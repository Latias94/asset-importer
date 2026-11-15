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
#include <assimp/config.h>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <mutex>
#include <unordered_set>
#include <string>
#include <utility>
#include <vector>

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

static bool metadata_can_be_copied(const aiMetadata* metadata, bool node_metadata) {
    std::vector<std::pair<const aiMetadata*, bool>> pending;
    if (metadata) {
        pending.emplace_back(metadata, node_metadata);
    }
    while (!pending.empty()) {
        const auto item = pending.back();
        pending.pop_back();
        const aiMetadata* current = item.first;
        if (item.second && current->mNumProperties == 0) {
            g_last_error_msg = "Cannot copy a node with empty metadata: Assimp would share and free the source metadata";
            return false;
        }
        if (current->mNumProperties && (!current->mKeys || !current->mValues)) {
            g_last_error_msg = "Cannot copy invalid metadata arrays";
            return false;
        }
        for (unsigned int i = 0; i < current->mNumProperties; ++i) {
            const aiMetadataEntry& entry = current->mValues[i];
            if (!entry.mData || entry.mType < AI_BOOL || entry.mType > AI_UINT32) {
                g_last_error_msg = "Cannot copy unassigned or unknown metadata with Assimp";
                return false;
            }
            if (item.second && (entry.mType == AI_INT64 || entry.mType == AI_UINT32)) {
                g_last_error_msg = "Cannot copy this node metadata integer type with Assimp";
                return false;
            }
            if (entry.mType == AI_AIMETADATA) {
                pending.emplace_back(static_cast<const aiMetadata*>(entry.mData), false);
            }
        }
    }
    return true;
}

static bool scene_can_be_copied(const aiScene* scene, bool include_scene_data = true) {
    // SceneCombiner stops copying aiAnimMesh channels at the first empty slot,
    // leaving later buffers shared with the source and freed by both owners.
    const auto channels_can_be_copied = [](const auto* mesh, const std::string& location) {
        bool gap = false;
        for (unsigned int channel = 0; channel < AI_MAX_NUMBER_OF_TEXTURECOORDS; ++channel) {
            if (!mesh->mTextureCoords[channel]) {
                gap = true;
            } else if (gap) {
                g_last_error_msg = location + " has sparse texture coordinate channel " +
                    std::to_string(channel) + "; Assimp cannot safely copy this scene";
                return false;
            }
        }
        gap = false;
        for (unsigned int channel = 0; channel < AI_MAX_NUMBER_OF_COLOR_SETS; ++channel) {
            if (!mesh->mColors[channel]) {
                gap = true;
            } else if (gap) {
                g_last_error_msg = location + " has sparse vertex color channel " +
                    std::to_string(channel) + "; Assimp cannot safely copy this scene";
                return false;
            }
        }
        return true;
    };

    for (unsigned int mesh_index = 0; mesh_index < scene->mNumMeshes; ++mesh_index) {
        const aiMesh* mesh = scene->mMeshes[mesh_index];
        const std::string location = "mesh " + std::to_string(mesh_index);
        // Prefer the morph target diagnostic when both layouts contain gaps.
        for (unsigned int morph = 0; morph < mesh->mNumAnimMeshes; ++morph) {
            if (!channels_can_be_copied(mesh->mAnimMeshes[morph],
                    location + ", morph target " + std::to_string(morph))) {
                return false;
            }
        }
        if (!channels_can_be_copied(mesh, location)) {
            return false;
        }
    }
    if (!include_scene_data) {
        return true;
    }
    if (!metadata_can_be_copied(scene->mMetaData, false)) {
        return false;
    }
    std::vector<const aiNode*> nodes;
    if (scene->mRootNode) {
        nodes.push_back(scene->mRootNode);
    }
    while (!nodes.empty()) {
        const aiNode* node = nodes.back();
        nodes.pop_back();
        if (!metadata_can_be_copied(node->mMetaData, true)) {
            return false;
        }
        for (unsigned int i = 0; i < node->mNumChildren; ++i) {
            nodes.push_back(node->mChildren[i]);
        }
    }
    for (unsigned int i = 0; i < scene->mNumAnimations; ++i) {
        if (scene->mAnimations[i]->mNumMeshChannels) {
            g_last_error_msg = "Cannot copy mesh animation channels: Assimp leaves their buffers shared with the source";
            return false;
        }
    }
    return true;
}

static bool postprocess_is_supported(unsigned int flags) {
    if (flags & aiProcess_SplitLargeMeshes) {
        g_last_error_msg = "SplitLargeMeshes is unsupported because Assimp 6.0.6 can retain freed meshes, write through null channel pointers, or fail to advance";
        return false;
    }
    if (flags & aiProcess_SplitByBoneCount) {
        g_last_error_msg = "SplitByBoneCount is unsupported because Assimp 6.0.6 can fail to advance, read null morph positions, or free meshes twice on failure";
        return false;
    }
    return true;
}

static bool global_scale_is_supported(const Assimp::Importer& importer, unsigned int flags) {
    if (!(flags & aiProcess_GlobalScale)) {
        return true;
    }
    ai_real scale = importer.GetPropertyFloat(AI_CONFIG_GLOBAL_SCALE_FACTOR_KEY, 1.0f);
    float importer_scale = importer.GetPropertyFloat(AI_CONFIG_APP_SCALE_KEY, 1.0f);
    scale *= importer_scale;
    if (!std::isfinite(scale) || scale == 0) {
        g_last_error_msg = "GlobalScale requires a finite, nonzero effective scale (GLOBAL_SCALE_FACTOR * APP_SCALE_FACTOR)";
        return false;
    }
    if (scale == 1.0f) {
        return true;
    }
    const aiScene* scene = importer.GetScene();
    for (unsigned int mesh_index = 0; mesh_index < scene->mNumMeshes; ++mesh_index) {
        const aiMesh* mesh = scene->mMeshes[mesh_index];
        for (unsigned int morph = 0; morph < mesh->mNumAnimMeshes; ++morph) {
            const aiAnimMesh* target = mesh->mAnimMeshes[morph];
            if (target->mNumVertices && !target->mVertices) {
                g_last_error_msg = "GlobalScale cannot scale mesh " + std::to_string(mesh_index) +
                    ", morph target " + std::to_string(morph) +
                    ": a nonempty morph target has no position channel";
                return false;
            }
        }
    }
    return true;
}

#ifndef ASSIMP_BUILD_NO_EXPORT
using ExportSceneCopy = std::unique_ptr<aiScene, decltype(&aiFreeScene)>;

static ExportSceneCopy normalize_export_scene(const aiScene* scene) {
    ExportSceneCopy copy(nullptr, aiFreeScene);
    if (!(scene->mFlags & AI_SCENE_FLAGS_NON_VERBOSE_FORMAT)) {
        return copy;
    }
    // Assimp's exporter skips vertex expansion when indices are already unique,
    // but leaves NON_VERBOSE_FORMAT set. Normal generation then rejects the scene.
    // Normalize only a private copy, preserving shared Rust views of the source.
    for (unsigned int m = 0; m < scene->mNumMeshes; ++m) {
        const aiMesh* mesh = scene->mMeshes[m];
        std::vector<unsigned char> seen(mesh->mNumVertices, 0);
        for (unsigned int f = 0; f < mesh->mNumFaces; ++f) {
            const aiFace& face = mesh->mFaces[f];
            for (unsigned int i = 0; i < face.mNumIndices; ++i) {
                const unsigned int index = face.mIndices[i];
                if (index >= seen.size() || seen[index]++) {
                    return copy;
                }
            }
        }
    }
    aiScene* native_copy = nullptr;
    aiCopyScene(scene, &native_copy);
    if (!native_copy) {
        throw std::bad_alloc();
    }
    copy.reset(native_copy);
    copy->mFlags &= ~AI_SCENE_FLAGS_NON_VERBOSE_FORMAT;
    return copy;
}
#endif

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
    if (!postprocess_is_supported(flags)) {
        return nullptr;
    }

    const bool guard_scene_copy = (flags & aiProcess_PreTransformVertices) &&
        importer.GetPropertyBool(AI_CONFIG_PP_PTV_KEEP_HIERARCHY, false);
    const bool guard_postprocess = guard_scene_copy || (flags & aiProcess_GlobalScale);
    // ReadFile validates before ScenePreprocessor. Defer the remaining steps so
    // scene-copy and scaling checks precede the post-processing pipeline.
    const unsigned int read_flags = guard_postprocess ? (flags & aiProcess_ValidateDataStructure) : flags;
    const aiScene* scene = nullptr;
    if (path) {
        scene = importer.ReadFile(path, read_flags);
    } else if (mem) {
        scene = importer.ReadFileFromMemory(mem, (size_t)mem_len, read_flags, hint ? hint : "");
    }

    // Assimp's void stage notifications ignore Update's return value. Let its
    // normal cleanup complete, then reject any scene produced after cancellation.
    if (progress && progress->cancelled) {
        g_last_error_msg = "Import cancelled by progress handler";
        return nullptr;
    }

    if (scene && guard_postprocess) {
        if ((guard_scene_copy && !scene_can_be_copied(scene, false)) ||
            !global_scale_is_supported(importer, flags)) {
            return nullptr;
        }
        scene = importer.ApplyPostProcessing(flags & ~aiProcess_ValidateDataStructure);
        if (progress && progress->cancelled) {
            g_last_error_msg = "Import cancelled by progress handler";
            return nullptr;
        }
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
        if (!postprocess_is_supported(flags)) {
            return false;
        }
        if ((flags & aiProcess_PreTransformVertices) &&
            owner->importer.GetPropertyBool(AI_CONFIG_PP_PTV_KEEP_HIERARCHY, false) &&
            !scene_can_be_copied(owner->importer.GetScene(), false)) {
            return false;
        }
        if (!global_scale_is_supported(owner->importer, flags)) {
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
    if (preprocessing != 0) {
        g_last_error_msg = "Export preprocessing is unsupported; prepare the scene with its importer before exporting";
        return aiReturn_FAILURE;
    }
    if (file_io && std::strcmp(format_id, "3mf") == 0) {
        g_last_error_msg = "3MF export does not support a custom file system because its ZIP writer writes directly to the host file system";
        return aiReturn_FAILURE;
    }
    if (!scene_can_be_copied(scene)) {
        return aiReturn_FAILURE;
    }

    auto normalized_scene = normalize_export_scene(scene);
    if (normalized_scene) {
        scene = normalized_scene.get();
    }
    Assimp::Exporter exporter;
    Assimp::ExportProperties export_props;

    if (file_io) {
        // Assimp::Exporter takes ownership of custom IO handlers via SetIOHandler().
        exporter.SetIOHandler(new RustIOSystem(file_io));
    }

    apply_export_properties(export_props, props, props_count);

    const aiReturn r = exporter.Export(scene, format_id, path, 0, &export_props);
    if (r != aiReturn_SUCCESS) {
        g_last_error_msg = exporter.GetErrorString();
    }
    return r;
#endif
    } catch (const std::bad_alloc& e) {
        set_exception_error("aiExportSceneExWithPropertiesRust", e);
        return aiReturn_OUTOFMEMORY;
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
    if (preprocessing != 0) {
        g_last_error_msg = "Export preprocessing is unsupported; prepare the scene with its importer before exporting";
        return nullptr;
    }
    if (std::strcmp(format_id, "3mf") == 0) {
        g_last_error_msg = "3MF blob export is unsupported because its ZIP writer writes directly to the host file system";
        return nullptr;
    }
    if (!scene_can_be_copied(scene)) {
        return nullptr;
    }

    auto normalized_scene = normalize_export_scene(scene);
    if (normalized_scene) {
        scene = normalized_scene.get();
    }
    Assimp::Exporter exporter;
    Assimp::ExportProperties export_props;

    apply_export_properties(export_props, props, props_count);

    const aiExportDataBlob* blob = exporter.ExportToBlob(scene, format_id, 0, &export_props);
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
