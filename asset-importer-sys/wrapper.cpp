/*
 * C++ wrapper implementations for Assimp bindings
 *
 * This file adds bridging for:
 * - Progress callbacks via Assimp::ProgressHandler
 * - Custom IO via Assimp::IOSystem wrapping C aiFileIO
 * - Property passing to Assimp::Importer / Assimp::ExportProperties
 */

#include "wrapper.h"

#include <assimp/Importer.hpp>
#ifndef ASSIMP_BUILD_NO_EXPORT
#include <assimp/Exporter.hpp>
#endif
#include <assimp/IOSystem.hpp>
#include <assimp/IOStream.hpp>
#include <assimp/ProgressHandler.hpp>
#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <mutex>
#include <unordered_set>
#include <string>

// Store a thread-local last-error message for the bridge
static thread_local std::string g_last_error_msg;

struct aiRustScene {
    Assimp::Importer importer;
};

namespace {

struct BridgeProgressHandler final : public Assimp::ProgressHandler {
    aiRustProgressCallback cb{nullptr};
    void* user{nullptr};
    bool cancelled{false};

    explicit BridgeProgressHandler(aiRustProgressCallback c, void* u) : cb(c), user(u) {}

    bool report(float percentage, const char* message) {
        if (cancelled) return false;
        if (cb && !cb(percentage, message, user)) cancelled = true;
        return !cancelled;
    }

    bool Update(float percentage = -1.f) override {
        // No message variant
        return report(percentage, nullptr);
    }

    void UpdateFileRead(int currentStep = 0, int numberOfSteps = 0) override {
        if (!cb) return;
        char buf[64];
        std::snprintf(buf, sizeof(buf), "read %d/%d", currentStep, numberOfSteps);
        (void)report(numberOfSteps ? (currentStep / (float)numberOfSteps) * 0.5f : 0.0f, buf);
    }

    void UpdatePostProcess(int currentStep = 0, int numberOfSteps = 0) override {
        if (!cb) return;
        char buf[64];
        std::snprintf(buf, sizeof(buf), "post %d/%d", currentStep, numberOfSteps);
        float f = numberOfSteps ? (currentStep / (float)numberOfSteps) : 1.0f;
        (void)report(f * 0.5f + 0.5f, buf);
    }

    void UpdateFileWrite(int currentStep = 0, int numberOfSteps = 0) override {
        if (!cb) return;
        char buf[64];
        std::snprintf(buf, sizeof(buf), "write %d/%d", currentStep, numberOfSteps);
        float f = numberOfSteps ? (currentStep / (float)numberOfSteps) : 1.0f;
        (void)report(f * 0.5f, buf);
    }
};

class RustIOSystem;

// IOStream wrapper
class RustIOStream final : public Assimp::IOStream {
public:
    RustIOSystem* m_system;
    const aiFileIO* m_fileio;
    aiFile* m_handle;

    RustIOStream(RustIOSystem* system, const aiFileIO* fio, aiFile* file)
        : m_system(system), m_fileio(fio), m_handle(file) {}
    ~RustIOStream() override;

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

    std::mutex m_streams_mutex;
    std::unordered_set<RustIOStream*> m_streams;

    explicit RustIOSystem(const aiFileIO* fio) : m_fileio(fio) {}
    ~RustIOSystem() override {
        // Some exporters throw while holding an unowned stream. Reclaim leftovers
        // while the Rust callback table is still alive.
        for (;;) {
            RustIOStream* stream;
            {
                std::lock_guard<std::mutex> lock(m_streams_mutex);
                if (m_streams.empty()) break;
                auto it = m_streams.begin();
                stream = *it;
                m_streams.erase(it);
                stream->m_system = nullptr;
            }
            delete stream;
        }
    }

    void forget(RustIOStream* stream) {
        std::lock_guard<std::mutex> lock(m_streams_mutex);
        m_streams.erase(stream);
    }

    // Try open/close to check existence
    bool Exists(const char* pFile) const override {
        if (!m_fileio || !m_fileio->OpenProc || !m_fileio->CloseProc) return false;
        aiFile* f = m_fileio->OpenProc((aiFileIO*)m_fileio, pFile, "rb");
        if (!f) return false;
        m_fileio->CloseProc((aiFileIO*)m_fileio, f);
        return true;
    }

    char getOsSeparator() const override {
#if defined(_WIN32)
        return '\\';
#else
        return '/';
#endif
    }

    Assimp::IOStream* Open(const char* pFile, const char* pMode = "rb") override {
        if (!m_fileio || !m_fileio->OpenProc || !m_fileio->CloseProc) return nullptr;
        aiFile* f = m_fileio->OpenProc((aiFileIO*)m_fileio, pFile, pMode);
        if (!f) return nullptr;
        std::unique_ptr<RustIOStream> stream;
        try {
            stream = std::make_unique<RustIOStream>(this, m_fileio, f);
        } catch (...) {
            m_fileio->CloseProc((aiFileIO*)m_fileio, f);
            throw;
        }
        {
            std::lock_guard<std::mutex> lock(m_streams_mutex);
            m_streams.insert(stream.get());
        }
        return stream.release();
    }

    void Close(Assimp::IOStream* pFile) override {
        delete pFile;
    }
};

RustIOStream::~RustIOStream() {
    // Assimp may delete streams directly instead of calling IOSystem::Close.
    if (m_system) m_system->forget(this);
    if (m_handle && m_fileio && m_fileio->CloseProc) {
        aiFile* handle = m_handle;
        m_handle = nullptr;
        m_fileio->CloseProc((aiFileIO*)m_fileio, handle);
    }
}

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
                if (p.matrix_value) {
                    importer.SetPropertyMatrix(p.name, *static_cast<const aiMatrix4x4*>(p.matrix_value));
                }
                break;
            default:
                break;
        }
    }
}

#ifndef ASSIMP_BUILD_NO_EXPORT
static void apply_export_properties(Assimp::ExportProperties& target, const aiRustProperty* props, size_t count) {
    if (!props) return;
    for (size_t i = 0; i < count; ++i) {
        const aiRustProperty& p = props[i];
        if (!p.name) continue;
        switch (p.kind) {
            case aiRustPropertyKind_Integer:
                target.SetPropertyInteger(p.name, p.int_value);
                break;
            case aiRustPropertyKind_Boolean:
                target.SetPropertyBool(p.name, p.int_value != 0);
                break;
            case aiRustPropertyKind_Float:
                target.SetPropertyFloat(p.name, p.float_value);
                break;
            case aiRustPropertyKind_String:
                target.SetPropertyString(p.name, p.string_value ? std::string(p.string_value) : std::string());
                break;
            case aiRustPropertyKind_Matrix4x4:
                if (p.matrix_value) {
                    target.SetPropertyMatrix(p.name, *static_cast<const aiMatrix4x4*>(p.matrix_value));
                }
                break;
            default:
                break;
        }
    }
}
#endif

static void set_bridge_error(const char* message) noexcept {
    try {
        g_last_error_msg = message ? message : "Unknown bridge error";
    } catch (...) {
        // Last-resort: do not allow error reporting itself to cross the C ABI.
    }
}

static void set_exception_error(const char* context, const std::exception& e) noexcept {
    try {
        g_last_error_msg = std::string(context) + " threw C++ exception: " + e.what();
    } catch (...) {
        set_bridge_error("Bridge threw C++ exception");
    }
}

static void set_unknown_exception_error(const char* context) noexcept {
    try {
        g_last_error_msg = std::string(context) + " threw unknown C++ exception";
    } catch (...) {
        set_bridge_error("Bridge threw unknown C++ exception");
    }
}

static aiRustScene* import_with_bridge(
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
    auto owner = std::make_unique<aiRustScene>();
    Assimp::Importer& importer = owner->importer;

    // IO bridge
    if (file_io) {
        // Assimp::Importer takes ownership of custom IO/progress handlers and deletes them.
        importer.SetIOHandler(new RustIOSystem(file_io));
    }

    // Progress bridge
    BridgeProgressHandler* progress = nullptr;
    if (progress_cb) {
        progress = new BridgeProgressHandler(progress_cb, progress_user);
        importer.SetProgressHandler(progress);
    }

    // Properties
    apply_properties(importer, props, props_count);

    const aiScene* scene = nullptr;
    if (path) {
        scene = importer.ReadFile(path, flags);
    } else if (mem) {
        scene = importer.ReadFileFromMemory(mem, (size_t)mem_len, flags, hint ? hint : "");
    }

    // Assimp's void stage notifications ignore Update's return value. Let its
    // normal cleanup complete, then reject any scene produced after cancellation.
    if (progress && progress->cancelled) {
        g_last_error_msg = "Import cancelled by progress handler";
        return nullptr;
    }

    if (!scene) {
        g_last_error_msg = importer.GetErrorString();
        return nullptr;
    }

    // Progress belongs to this import call. Custom IO stays with the scene owner
    // so later post-processing cannot fall back to the host file system.
    if (progress) {
        progress->cb = nullptr;
        progress->user = nullptr;
    }
    return owner.release();
}

} // namespace

extern "C" {

// Implementation of functions declared in wrapper.h
struct aiRustScene* aiImportFileRust(
    const char* path,
    unsigned int flags,
    const struct aiFileIO* file_io,
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb,
    void* progress_user)
{
    g_last_error_msg.clear();
    try {
        if (!path) {
            g_last_error_msg = "Path is null";
            return nullptr;
        }
        return import_with_bridge(path, nullptr, 0u, flags, file_io, props, props_count, progress_cb, progress_user, nullptr);
    } catch (const std::exception& e) {
        set_exception_error("aiImportFileRust", e);
        return nullptr;
    } catch (...) {
        set_unknown_exception_error("aiImportFileRust");
        return nullptr;
    }
}

struct aiRustScene* aiImportFileFromMemoryRust(
    const char* data,
    unsigned int length,
    unsigned int flags,
    const char* hint,
    const struct aiFileIO* file_io,
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb,
    void* progress_user)
{
    g_last_error_msg.clear();
    try {
        if (!data || length == 0u) {
            g_last_error_msg = "Memory buffer is empty";
            return nullptr;
        }
        return import_with_bridge(nullptr, data, length, flags, file_io, props, props_count, progress_cb, progress_user, hint);
    } catch (const std::exception& e) {
        set_exception_error("aiImportFileFromMemoryRust", e);
        return nullptr;
    } catch (...) {
        set_unknown_exception_error("aiImportFileFromMemoryRust");
        return nullptr;
    }
}

const struct aiScene* aiGetSceneRust(const struct aiRustScene* owner) {
    return owner ? owner->importer.GetScene() : nullptr;
}

void aiReleaseSceneRust(struct aiRustScene* owner) {
    delete owner;
}

bool aiApplyPostProcessingRust(struct aiRustScene* owner, unsigned int flags) {
    g_last_error_msg.clear();
    try {
        if (!owner || !owner->importer.GetScene()) {
            set_bridge_error("Scene owner is empty");
            return false;
        }
        if (!owner->importer.ApplyPostProcessing(flags)) {
            set_bridge_error(owner->importer.GetErrorString());
            return false;
        }
        return true;
    } catch (const std::exception& e) {
        set_exception_error("aiApplyPostProcessingRust", e);
        return false;
    } catch (...) {
        set_unknown_exception_error("aiApplyPostProcessingRust");
        return false;
    }
}

bool aiGetMemoryRequirementsRust(const struct aiRustScene* owner, struct aiMemoryInfo* info) {
    g_last_error_msg.clear();
    try {
        if (!owner || !owner->importer.GetScene() || !info) {
            set_bridge_error("Scene owner or memory info is empty");
            return false;
        }
        owner->importer.GetMemoryRequirements(*info);
        return true;
    } catch (const std::exception& e) {
        set_exception_error("aiGetMemoryRequirementsRust", e);
        return false;
    } catch (...) {
        set_unknown_exception_error("aiGetMemoryRequirementsRust");
        return false;
    }
}

enum aiReturn aiExportSceneExWithPropertiesRust(
    const struct aiScene* scene,
    const char* format_id,
    const char* path,
    const struct aiFileIO* file_io,
    unsigned int preprocessing,
    const struct aiRustProperty* props,
    size_t props_count)
{
    g_last_error_msg.clear();
    try {

#ifdef ASSIMP_BUILD_NO_EXPORT
    (void)scene;
    (void)format_id;
    (void)path;
    (void)file_io;
    (void)preprocessing;
    (void)props;
    (void)props_count;
    g_last_error_msg = "Assimp was built without export support (ASSIMP_BUILD_NO_EXPORT)";
    return aiReturn_FAILURE;
#else
    if (!scene) {
        g_last_error_msg = "Scene is null";
        return aiReturn_FAILURE;
    }
    if (!format_id || !path) {
        g_last_error_msg = "Format ID or path is null";
        return aiReturn_FAILURE;
    }

    Assimp::Exporter exporter;
    Assimp::ExportProperties export_props;

    if (file_io) {
        // Assimp::Exporter takes ownership of custom IO handlers via SetIOHandler().
        exporter.SetIOHandler(new RustIOSystem(file_io));
    }

    apply_export_properties(export_props, props, props_count);

    const aiReturn r = exporter.Export(scene, format_id, path, preprocessing, &export_props);
    if (r != aiReturn_SUCCESS) {
        g_last_error_msg = exporter.GetErrorString();
    }
    return r;
#endif
    } catch (const std::exception& e) {
        set_exception_error("aiExportSceneExWithPropertiesRust", e);
        return aiReturn_FAILURE;
    } catch (...) {
        set_unknown_exception_error("aiExportSceneExWithPropertiesRust");
        return aiReturn_FAILURE;
    }
}

const struct aiExportDataBlob* aiExportSceneToBlobWithPropertiesRust(
    const struct aiScene* scene,
    const char* format_id,
    unsigned int preprocessing,
    const struct aiRustProperty* props,
    size_t props_count)
{
    g_last_error_msg.clear();
    try {

#ifdef ASSIMP_BUILD_NO_EXPORT
    (void)scene;
    (void)format_id;
    (void)preprocessing;
    (void)props;
    (void)props_count;
    g_last_error_msg = "Assimp was built without export support (ASSIMP_BUILD_NO_EXPORT)";
    return nullptr;
#else
    if (!scene) {
        g_last_error_msg = "Scene is null";
        return nullptr;
    }
    if (!format_id) {
        g_last_error_msg = "Format ID is null";
        return nullptr;
    }

    Assimp::Exporter exporter;
    Assimp::ExportProperties export_props;

    apply_export_properties(export_props, props, props_count);

    const aiExportDataBlob* blob = exporter.ExportToBlob(scene, format_id, preprocessing, &export_props);
    if (!blob) {
        g_last_error_msg = exporter.GetErrorString();
        return nullptr;
    }

    return exporter.GetOrphanedBlob();
#endif
    } catch (const std::exception& e) {
        set_exception_error("aiExportSceneToBlobWithPropertiesRust", e);
        return nullptr;
    } catch (...) {
        set_unknown_exception_error("aiExportSceneToBlobWithPropertiesRust");
        return nullptr;
    }
}

const char* aiGetLastErrorStringRust(void) {
    return g_last_error_msg.empty() ? nullptr : g_last_error_msg.c_str();
}

} // extern "C"
