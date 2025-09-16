/*
 * C++ wrapper implementations for Assimp bindings
 *
 * This file adds bridging for:
 * - Progress callbacks via Assimp::ProgressHandler
 * - Custom IO via Assimp::IOSystem wrapping C aiFileIO
 * - Property passing to Assimp::Importer
 */

#include "wrapper.h"

#include <assimp/Importer.hpp>
#include <assimp/IOSystem.hpp>
#include <assimp/IOStream.hpp>
#include <assimp/cexport.h> // aiCopyScene
#include <cstring>
#include <string>
#include <memory>
#include <mutex>

// Store a thread-local last-error message for the bridge
static thread_local std::string g_last_error_msg;

namespace {

struct BridgeProgressHandler final : public Assimp::ProgressHandler {
    aiRustProgressCallback cb{nullptr};
    void* user{nullptr};

    explicit BridgeProgressHandler(aiRustProgressCallback c, void* u) : cb(c), user(u) {}

    bool Update(float percentage = -1.f) override {
        if (!cb) return true;
        // No message variant
        return cb(percentage, nullptr, user);
    }

    void UpdateFileRead(int currentStep = 0, int numberOfSteps = 0) override {
        if (!cb) return;
        char buf[64];
        std::snprintf(buf, sizeof(buf), "read %d/%d", currentStep, numberOfSteps);
        (void)cb(numberOfSteps ? (currentStep / (float)numberOfSteps) * 0.5f : 0.0f, buf, user);
    }

    void UpdatePostProcess(int currentStep = 0, int numberOfSteps = 0) override {
        if (!cb) return;
        char buf[64];
        std::snprintf(buf, sizeof(buf), "post %d/%d", currentStep, numberOfSteps);
        float f = numberOfSteps ? (currentStep / (float)numberOfSteps) : 1.0f;
        (void)cb(f * 0.5f + 0.5f, buf, user);
    }

    void UpdateFileWrite(int currentStep = 0, int numberOfSteps = 0) override {
        if (!cb) return;
        char buf[64];
        std::snprintf(buf, sizeof(buf), "write %d/%d", currentStep, numberOfSteps);
        float f = numberOfSteps ? (currentStep / (float)numberOfSteps) : 1.0f;
        (void)cb(f * 0.5f, buf, user);
    }
};

// IOStream wrapper
class RustIOStream final : public Assimp::IOStream {
public:
    const aiFileIO* m_fileio;
    aiFile* m_handle;

    RustIOStream(const aiFileIO* fio, aiFile* file) : m_fileio(fio), m_handle(file) {}
    ~RustIOStream() override = default;

    size_t Read(void* pvBuffer, size_t pSize, size_t pCount) override {
        if (!m_handle || !m_handle->ReadProc) return 0u;
        return m_handle->ReadProc(m_handle, (char*)pvBuffer, pSize, pCount);
    }

    size_t Write(const void* pvBuffer, size_t pSize, size_t pCount) override {
        if (!m_handle || !m_handle->WriteProc) return 0u;
        return m_handle->WriteProc(m_handle, (const char*)pvBuffer, pSize, pCount);
    }

    aiReturn Seek(size_t pOffset, aiOrigin pOrigin) override {
        if (!m_handle || !m_handle->SeekProc) return aiReturn_FAILURE;
        return m_handle->SeekProc(m_handle, pOffset, pOrigin);
    }

    size_t Tell() const override {
        if (!m_handle || !m_handle->TellProc) return 0u;
        return m_handle->TellProc(m_handle);
    }

    size_t FileSize() const override {
        if (!m_handle || !m_handle->FileSizeProc) return 0u;
        return m_handle->FileSizeProc(m_handle);
    }

    void Flush() override {
        if (m_handle && m_handle->FlushProc) m_handle->FlushProc(m_handle);
    }
};

// IOSystem wrapper bridging aiFileIO
class RustIOSystem final : public Assimp::IOSystem {
public:
    const aiFileIO* m_fileio; // non-owning

    explicit RustIOSystem(const aiFileIO* fio) : m_fileio(fio) {}
    ~RustIOSystem() override = default;

    // Try open/close to check existence
    bool Exists(const char* pFile) const override {
        if (!m_fileio || !m_fileio->OpenProc || !m_fileio->CloseProc) return false;
        aiFile* f = m_fileio->OpenProc((aiFileIO*)m_fileio, pFile, "rb");
        if (!f) return false;
        m_fileio->CloseProc((aiFileIO*)m_fileio, f);
        return true;
    }

    char getOsSeparator() const override { return '/'; }

    Assimp::IOStream* Open(const char* pFile, const char* pMode = "rb") override {
        if (!m_fileio || !m_fileio->OpenProc) return nullptr;
        aiFile* f = m_fileio->OpenProc((aiFileIO*)m_fileio, pFile, pMode);
        if (!f) return nullptr;
        return new RustIOStream(m_fileio, f);
    }

    void Close(Assimp::IOStream* pFile) override {
        if (!pFile) return;
        auto* s = dynamic_cast<RustIOStream*>(pFile);
        if (s && m_fileio && m_fileio->CloseProc && s->m_handle) {
            m_fileio->CloseProc((aiFileIO*)m_fileio, s->m_handle);
            s->m_handle = nullptr;
        }
        delete pFile;
    }
};

static void apply_properties(Assimp::Importer& importer, const aiRustProperty* props, size_t count) {
    if (!props) return;
    for (size_t i = 0; i < count; ++i) {
        const aiRustProperty& p = props[i];
        if (!p.name) continue;
        switch (p.kind) {
            case aiRustPropertyKind_Integer:
                importer.SetPropertyInteger(p.name, p.int_value);
                break;
            case aiRustPropertyKind_Boolean:
                importer.SetPropertyBool(p.name, p.int_value != 0);
                break;
            case aiRustPropertyKind_Float:
                importer.SetPropertyFloat(p.name, p.float_value);
                break;
            case aiRustPropertyKind_String:
                importer.SetPropertyString(p.name, p.string_value ? std::string(p.string_value) : std::string());
                break;
            case aiRustPropertyKind_Matrix4x4:
                importer.SetPropertyMatrix(p.name, p.matrix_value);
                break;
            default:
                break;
        }
    }
}

static const aiScene* import_with_bridge(
    const char* path,
    const char* mem,
    unsigned int mem_len,
    unsigned int flags,
    const aiFileIO* file_io,
    const aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb,
    void* progress_user,
    const char* hint)
{
    Assimp::Importer importer;

    // IO bridge
    std::unique_ptr<RustIOSystem> io_guard;
    if (file_io) {
        io_guard = std::make_unique<RustIOSystem>(file_io);
        importer.SetIOHandler(io_guard.get());
    }

    // Progress bridge
    std::unique_ptr<BridgeProgressHandler> ph;
    if (progress_cb) {
        ph = std::make_unique<BridgeProgressHandler>(progress_cb, progress_user);
        importer.SetProgressHandler(ph.get());
    }

    // Properties
    apply_properties(importer, props, props_count);

    const aiScene* scene = nullptr;
    if (path) {
        scene = importer.ReadFile(path, flags);
    } else if (mem) {
        scene = importer.ReadFileFromMemory(mem, (size_t)mem_len, flags, hint ? hint : "");
    }

    if (!scene) {
        g_last_error_msg = importer.GetErrorString();
        return nullptr;
    }

    // Deep copy to decouple from Importer lifetime
    aiScene* out = nullptr;
    aiCopyScene(scene, &out);
    if (!out) {
        g_last_error_msg = "aiCopyScene returned null";
        return nullptr;
    }
    return out;
}

} // namespace

extern "C" {

const struct aiScene* aiImportFileExWithProgressRust(
    const char* path,
    unsigned int flags,
    const struct aiFileIO* file_io,
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb,
    void* progress_user)
{
    g_last_error_msg.clear();
    if (!path) {
        g_last_error_msg = "Path is null";
        return nullptr;
    }
    return import_with_bridge(path, nullptr, 0u, flags, file_io, props, props_count, progress_cb, progress_user, nullptr);
}

const struct aiScene* aiImportFileFromMemoryWithProgressRust(
    const char* data,
    unsigned int length,
    unsigned int flags,
    const char* hint,
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb,
    void* progress_user)
{
    g_last_error_msg.clear();
    if (!data || length == 0u) {
        g_last_error_msg = "Memory buffer is empty";
        return nullptr;
    }
    return import_with_bridge(nullptr, data, length, flags, nullptr, props, props_count, progress_cb, progress_user, hint);
}

const char* aiGetLastErrorStringRust(void) {
    return g_last_error_msg.empty() ? nullptr : g_last_error_msg.c_str();
}

} // extern "C"
