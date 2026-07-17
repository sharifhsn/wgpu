use alloc::{string::String, sync::Arc, vec::Vec};
use core::{future::Future, marker::PhantomData, num::NonZeroU32};

use crate::*;

/// Handle to a compiled shader module.
///
/// A `ShaderModule` represents a compiled shader module on the GPU. It can be created by passing
/// source code to [`Device::create_shader_module`]. MSL shader, SPIR-V binary, or Deko3D DKSH
/// binary can also be passed directly using [`Device::create_shader_module_passthrough`]. Shader
/// modules are used to define
/// programmable stages of a pipeline.
///
/// Corresponds to [WebGPU `GPUShaderModule`](https://gpuweb.github.io/gpuweb/#shader-module).
#[derive(Debug, Clone)]
pub struct ShaderModule {
    pub(crate) inner: dispatch::DispatchShaderModule,
    pub(crate) deko3d_wgsl: Option<Arc<[u8]>>,
}

/// Stage for which a Deko3D offline artifact is requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Deko3dWgslArtifactStage {
    /// Vertex stage.
    Vertex,
    /// Fragment stage.
    Fragment,
    /// Compute stage.
    Compute,
}

/// Pipeline-layout descriptor count for one runtime-sized WGSL resource binding array.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Deko3dWgslBindingArraySize {
    /// Bind-group index.
    pub group: u32,
    /// Binding index within the group.
    pub binding: u32,
    /// Number of descriptors declared by the bind-group layout.
    pub count: u32,
}

/// Exact WGSL and pipeline-stage metadata used to resolve a Deko3D DKSH artifact.
#[derive(Clone, Copy, Debug)]
pub struct Deko3dWgslArtifactRequest<'a> {
    /// The exact final WGSL bytes passed to `create_shader_module`.
    pub wgsl: &'a [u8],
    /// SHA-256 digest of [`Self::wgsl`].
    pub wgsl_sha256: [u8; 32],
    /// Pipeline stage requiring the artifact.
    pub stage: Deko3dWgslArtifactStage,
    /// Requested pipeline entry point, with `main` substituted for an omitted entry point.
    pub entry_point: &'a str,
    /// Pipeline-overridable constants. Duplicate names are permitted and the last value wins.
    pub constants: &'a [(&'a str, f64)],
    /// Whether workgroup-scoped memory must be initialized to zero for this stage.
    pub zero_initialize_workgroup_memory: bool,
    /// Render-pipeline view mask. Multiview vertex artifacts must write `gl_Layer` and read the
    /// current view index from Deko3D vertex uniform-buffer slot 14. Compute and fragment artifact
    /// requests always use `None`.
    pub multiview_mask: Option<NonZeroU32>,
    /// Descriptor counts from the explicit pipeline layout. Runtime-sized WGSL
    /// `binding_array<T>` declarations require the matching entry.
    pub binding_array_sizes: &'a [Deko3dWgslBindingArraySize],
}

/// A trusted source of offline-compiled Deko3D DKSH artifacts.
pub trait Deko3dWgslArtifactProvider: Send + Sync {
    /// Resolves an exact WGSL/stage/entry-point/options request to validated DKSH bytes.
    fn resolve(&self, request: Deko3dWgslArtifactRequest<'_>) -> Result<Arc<[u8]>, String>;
}

/// Failure to install a Deko3D WGSL artifact provider or compile WGSL for Deko3D.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Deko3dWgslArtifactError {
    /// A provider was already installed for this device.
    AlreadyInstalled,
    /// A Deko3D WGSL module required a provider but none is installed.
    NotInstalled,
    /// The built-in Deko3D shader compiler rejected the request.
    Compiler(String),
    /// The trusted provider rejected an artifact request.
    Provider(String),
    /// The trusted provider returned malformed DKSH bytes.
    InvalidDksh(&'static str),
}

impl core::fmt::Display for Deko3dWgslArtifactError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyInstalled => {
                f.write_str("a Deko3D WGSL artifact provider is already installed")
            }
            Self::NotInstalled => f.write_str("no Deko3D WGSL artifact provider is installed"),
            Self::Compiler(message) => {
                write!(f, "Deko3D WGSL compilation failed: {message}")
            }
            Self::Provider(message) => write!(
                f,
                "Deko3D WGSL artifact provider rejected the request: {message}"
            ),
            Self::InvalidDksh(message) => write!(
                f,
                "Deko3D WGSL artifact provider returned invalid DKSH: {message}"
            ),
        }
    }
}

impl core::error::Error for Deko3dWgslArtifactError {}
#[cfg(send_sync)]
static_assertions::assert_impl_all!(ShaderModule: Send, Sync);

crate::cmp::impl_eq_ord_hash_proxy!(ShaderModule => .inner);

impl ShaderModule {
    /// Get the compilation info for the shader module.
    pub fn get_compilation_info(&self) -> impl Future<Output = CompilationInfo> + WasmNotSend {
        self.inner.get_compilation_info()
    }

    #[cfg(custom)]
    /// Returns custom implementation of ShaderModule (if custom backend and is internally T)
    pub fn as_custom<T: custom::ShaderModuleInterface>(&self) -> Option<&T> {
        self.inner.as_custom()
    }
}

/// Compilation information for a shader module.
///
/// Corresponds to [WebGPU `GPUCompilationInfo`](https://gpuweb.github.io/gpuweb/#gpucompilationinfo).
/// The source locations use bytes, and index a UTF-8 encoded string.
#[derive(Debug, Clone)]
pub struct CompilationInfo {
    /// The messages from the shader compilation process.
    pub messages: Vec<CompilationMessage>,
}

/// A single message from the shader compilation process.
///
/// Roughly corresponds to [`GPUCompilationMessage`](https://www.w3.org/TR/webgpu/#gpucompilationmessage),
/// except that the location uses UTF-8 for all positions.
#[derive(Debug, Clone)]
pub struct CompilationMessage {
    /// The text of the message.
    pub message: String,
    /// The type of the message.
    pub message_type: CompilationMessageType,
    /// Where in the source code the message points at.
    pub location: Option<SourceLocation>,
}

/// The type of a compilation message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationMessageType {
    /// An error message.
    Error,
    /// A warning message.
    Warning,
    /// An informational message.
    Info,
}

/// A human-readable representation for a span, tailored for text source.
///
/// Roughly corresponds to the positional members of [`GPUCompilationMessage`][gcm] from
/// the WebGPU specification, except
/// - `offset` and `length` are in bytes (UTF-8 code units), instead of UTF-16 code units.
/// - `line_position` is in bytes (UTF-8 code units), and is usually not directly intended for humans.
///
/// [gcm]: https://www.w3.org/TR/webgpu/#gpucompilationmessage
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    /// 1-based line number.
    pub line_number: u32,
    /// 1-based column in code units (in bytes) of the start of the span.
    /// Remember to convert accordingly when displaying to the user.
    pub line_position: u32,
    /// 0-based Offset in code units (in bytes) of the start of the span.
    pub offset: u32,
    /// Length in code units (in bytes) of the span.
    pub length: u32,
}

#[cfg(all(feature = "wgsl", wgpu_core))]
impl From<crate::naga::error::ShaderError<crate::naga::front::wgsl::ParseError>>
    for CompilationInfo
{
    fn from(value: crate::naga::error::ShaderError<crate::naga::front::wgsl::ParseError>) -> Self {
        use alloc::{string::ToString, vec};
        CompilationInfo {
            messages: vec![CompilationMessage {
                message: value.to_string(),
                message_type: CompilationMessageType::Error,
                location: value.inner.location(&value.source).map(Into::into),
            }],
        }
    }
}
#[cfg(feature = "glsl")]
impl From<naga::error::ShaderError<naga::front::glsl::ParseErrors>> for CompilationInfo {
    fn from(value: naga::error::ShaderError<naga::front::glsl::ParseErrors>) -> Self {
        use alloc::string::ToString;
        let messages = value
            .inner
            .errors
            .into_iter()
            .map(|err| CompilationMessage {
                message: err.to_string(),
                message_type: CompilationMessageType::Error,
                location: err.location(&value.source).map(Into::into),
            })
            .collect();
        CompilationInfo { messages }
    }
}

#[cfg(feature = "spirv")]
impl From<naga::error::ShaderError<naga::front::spv::Error>> for CompilationInfo {
    fn from(value: naga::error::ShaderError<naga::front::spv::Error>) -> Self {
        use alloc::{string::ToString, vec};
        CompilationInfo {
            messages: vec![CompilationMessage {
                message: value.to_string(),
                message_type: CompilationMessageType::Error,
                location: None,
            }],
        }
    }
}

#[cfg(any(wgpu_core, naga))]
impl
    From<
        crate::naga::error::ShaderError<crate::naga::WithSpan<crate::naga::valid::ValidationError>>,
    > for CompilationInfo
{
    fn from(
        value: crate::naga::error::ShaderError<
            crate::naga::WithSpan<crate::naga::valid::ValidationError>,
        >,
    ) -> Self {
        use alloc::{string::ToString, vec};
        CompilationInfo {
            messages: vec![CompilationMessage {
                message: value.to_string(),
                message_type: CompilationMessageType::Error,
                location: value.inner.location(&value.source).map(Into::into),
            }],
        }
    }
}

#[cfg(any(wgpu_core, naga))]
impl From<crate::naga::SourceLocation> for SourceLocation {
    fn from(value: crate::naga::SourceLocation) -> Self {
        SourceLocation {
            length: value.length,
            offset: value.offset,
            line_number: value.line_number,
            line_position: value.line_position,
        }
    }
}

/// Source of a shader module.
///
/// The source will be parsed and validated.
///
/// Any necessary shader translation (e.g. from WGSL to SPIR-V or vice versa)
/// will be done internally by wgpu.
///
/// This type is unique to the Rust API of `wgpu`. In the WebGPU specification,
/// only WGSL source code strings are accepted.
#[cfg_attr(feature = "naga-ir", expect(clippy::large_enum_variant))]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ShaderSource<'a> {
    /// SPIR-V module represented as a slice of words.
    ///
    /// See also: [`util::make_spirv`], [`include_spirv`]
    #[cfg(feature = "spirv")]
    SpirV(alloc::borrow::Cow<'a, [u32]>),
    /// GLSL module as a string slice.
    ///
    /// Note: GLSL is not yet fully supported and must be a specific ShaderStage.
    #[cfg(feature = "glsl")]
    Glsl {
        /// The source code of the shader.
        shader: alloc::borrow::Cow<'a, str>,
        /// The shader stage that the shader targets. For example, `naga::ShaderStage::Vertex`
        stage: naga::ShaderStage,
        /// Key-value pairs to represent defines sent to the glsl preprocessor.
        ///
        /// If the same name is defined multiple times, the last value is used.
        defines: &'a [(&'a str, &'a str)],
    },
    /// WGSL module as a string slice.
    #[cfg(feature = "wgsl")]
    Wgsl(alloc::borrow::Cow<'a, str>),
    /// Naga module.
    #[cfg(feature = "naga-ir")]
    Naga(alloc::borrow::Cow<'static, naga::Module>),
    /// Dummy variant because `Naga` doesn't have a lifetime and without enough active features it
    /// could be the last one active.
    #[doc(hidden)]
    Dummy(PhantomData<&'a ()>),
}
static_assertions::assert_impl_all!(ShaderSource<'_>: Send, Sync);

/// Descriptor for use with [`Device::create_shader_module`].
///
/// Corresponds to [WebGPU `GPUShaderModuleDescriptor`](
/// https://gpuweb.github.io/gpuweb/#dictdef-gpushadermoduledescriptor).
#[derive(Clone, Debug)]
pub struct ShaderModuleDescriptor<'a> {
    /// Debug label of the shader module. This will show up in graphics debuggers for easy identification.
    pub label: Label<'a>,
    /// Source code for the shader.
    pub source: ShaderSource<'a>,
}
static_assertions::assert_impl_all!(ShaderModuleDescriptor<'_>: Send, Sync);

/// Descriptor for a shader module given by any of several sources.
/// At least one of the shader types that may be used by the backend must be `Some`
///
/// This type is unique to the Rust API of `wgpu`. In the WebGPU specification,
/// only WGSL source code strings are accepted.
pub type ShaderModuleDescriptorPassthrough<'a> =
    wgt::CreateShaderModuleDescriptorPassthrough<'a, Label<'a>>;

/// Descriptor for a Deko3D shader module backed by offline-compiled DKSH bytes.
#[derive(Clone, Debug)]
pub struct Deko3dDkshShaderModuleDescriptor<'a> {
    /// Debug label of the shader module. This will show up in graphics debuggers for easy identification.
    pub label: Label<'a>,
    /// Number of workgroups in each dimension x, y and z. Unused for graphics shaders.
    pub num_workgroups: (u32, u32, u32),
    /// Binary Deko3D DKSH data, as produced by the offline deko3d shader compiler.
    pub dksh: alloc::borrow::Cow<'a, [u8]>,
}
static_assertions::assert_impl_all!(Deko3dDkshShaderModuleDescriptor<'_>: Send, Sync);

impl<'a> Deko3dDkshShaderModuleDescriptor<'a> {
    /// Constructs a descriptor for one offline-compiled entry point.
    ///
    /// Deko3D DKSH already contains the selected graphics or compute entry point, so the name is
    /// retained for source compatibility with newer wgpu artifact tooling but is not encoded
    /// separately by the wgpu 29 passthrough descriptor.
    pub fn single_entry(
        label: Label<'a>,
        _entry_point: &'a str,
        num_workgroups: (u32, u32, u32),
        dksh: alloc::borrow::Cow<'a, [u8]>,
    ) -> Self {
        Self {
            label,
            num_workgroups,
            dksh,
        }
    }
}
