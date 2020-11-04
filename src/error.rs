//! Error types and conversion functions.


use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use std::sync::{Arc, PoisonError};
use std::sync::mpsc::SendError;

/// An enum containing all kinds of game framework errors.
#[derive(Debug, Clone)]
pub enum GameError {
    /// An error in the filesystem layout
    FilesystemError(String),
    /// An error in the config file
    ConfigError(String),
    /// Happens when an `winit::EventsLoopProxy` attempts to
    /// wake up an `winit::EventsLoop` that no longer exists.
    EventLoopError(String),
    /// An error trying to load a resource, such as getting an invalid image file.
    ResourceLoadError(String),
    /// Unable to find a resource; the `Vec` is the paths it searched for and associated errors
    ResourceNotFound(String, Vec<(std::path::PathBuf, GameError)>),
    /// Something went wrong in the renderer
    RenderError(String),
    /// Something went wrong in the audio playback
    AudioError(String),
    /// Something went wrong trying to set or get window properties.
    WindowError(String),
    /// Something went wrong trying to create a window
    WindowCreationError(Arc<glutin::CreationError>),
    /// Something went wrong trying to read from a file
    IOError(Arc<std::io::Error>),
    /// Something went wrong trying to load/render a font
    FontError(String),
    /// Something went wrong applying video settings.
    VideoError(String),
    /// Something went wrong compiling shaders
    ShaderProgramError(gfx::shade::ProgramError),
    /// Something went wrong with the `gilrs` gamepad-input library.
    GamepadError(String),
    /// Something went wrong with the `lyon` shape-tesselation library.
    LyonError(String),
    /// Something went wrong while parsing something.
    ParseError(String),
    /// Something went wrong while converting a value.
    InvalidValue(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GameError::ConfigError(ref s) => write!(f, "Config error: {}", s),
            GameError::ResourceLoadError(ref s) => write!(f, "Error loading resource: {}", s),
            GameError::ResourceNotFound(ref s, ref paths) => write!(
                f,
                "Resource not found: {}, searched in paths {:?}",
                s, paths
            ),
            GameError::WindowError(ref e) => write!(f, "Window creation error: {}", e),
            _ => write!(f, "GameError {:?}", self),
        }
    }
}

impl Error for GameError {
    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            GameError::WindowCreationError(ref e) => Some(&**e),
            GameError::IOError(ref e) => Some(&**e),
            GameError::ShaderProgramError(ref e) => Some(e),
            _ => None,
        }
    }
}

/// A convenient result type consisting of a return type and a `GameError`
pub type GameResult<T = ()> = Result<T, GameError>;

impl From<std::io::Error> for GameError {
    fn from(e: std::io::Error) -> GameError {
        GameError::IOError(Arc::new(e))
    }
}

impl From<image::ImageError> for GameError {
    fn from(e: image::ImageError) -> GameError {
        let errstr = format!("Image load error: {}", e);
        GameError::ResourceLoadError(errstr)
    }
}

impl From<gfx::PipelineStateError<std::string::String>> for GameError {
    fn from(e: gfx::PipelineStateError<std::string::String>) -> GameError {
        let errstr = format!(
            "Error constructing pipeline!\nThis should probably not be \
             happening; it probably means an error in a shader or \
             something.\nError was: {:?}",
            e
        );
        GameError::VideoError(errstr)
    }
}

impl From<gfx::mapping::Error> for GameError {
    fn from(e: gfx::mapping::Error) -> GameError {
        let errstr = format!("Buffer mapping error: {:?}", e);
        GameError::VideoError(errstr)
    }
}

impl From<std::string::FromUtf8Error> for GameError {
    fn from(e: FromUtf8Error) -> Self {
        let errstr = format!("UTF-8 decoding error: {:?}", e);
        GameError::ConfigError(errstr)
    }
}

impl<S, D> From<gfx::CopyError<S, D>> for GameError
    where
        S: fmt::Debug,
        D: fmt::Debug,
{
    fn from(e: gfx::CopyError<S, D>) -> GameError {
        let errstr = format!("Memory copy error: {:?}", e);
        GameError::VideoError(errstr)
    }
}

impl From<gfx::CombinedError> for GameError {
    fn from(e: gfx::CombinedError) -> GameError {
        let errstr = format!("Texture+view load error: {}", e);
        GameError::VideoError(errstr)
    }
}

impl From<gfx::texture::CreationError> for GameError {
    fn from(e: gfx::texture::CreationError) -> GameError {
        gfx::CombinedError::from(e).into()
    }
}

impl From<gfx::ResourceViewError> for GameError {
    fn from(e: gfx::ResourceViewError) -> GameError {
        gfx::CombinedError::from(e).into()
    }
}

impl From<gfx::TargetViewError> for GameError {
    fn from(e: gfx::TargetViewError) -> GameError {
        gfx::CombinedError::from(e).into()
    }
}

impl<T> From<gfx::UpdateError<T>> for GameError
    where
        T: fmt::Debug + fmt::Display + 'static,
{
    fn from(e: gfx::UpdateError<T>) -> GameError {
        let errstr = format!("Buffer update error: {}", e);
        GameError::VideoError(errstr)
    }
}

impl From<gfx::shade::ProgramError> for GameError {
    fn from(e: gfx::shade::ProgramError) -> GameError {
        GameError::ShaderProgramError(e)
    }
}

impl<T> From<winit::event_loop::EventLoopClosed<T>> for GameError {
    fn from(_: winit::event_loop::EventLoopClosed<T>) -> GameError {
        let e = "An event loop proxy attempted to wake up an event loop that no longer exists."
            .to_owned();
        GameError::EventLoopError(e)
    }
}

impl From<glutin::CreationError> for GameError {
    fn from(s: glutin::CreationError) -> GameError {
        GameError::WindowCreationError(Arc::new(s))
    }
}

impl From<glutin::ContextError> for GameError {
    fn from(s: glutin::ContextError) -> GameError {
        GameError::RenderError(format!("OpenGL context error: {}", s))
    }
}

impl From<gilrs::Error> for GameError {
    fn from(s: gilrs::Error) -> GameError {
        let errstr = format!("Gamepad error: {}", s);
        GameError::GamepadError(errstr)
    }
}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for GameError {
    fn from(e: jni::errors::Error) -> GameError {
        GameError::WindowError(e.to_string())
    }
}

impl From<lyon::lyon_tessellation::TessellationError> for GameError {
    fn from(s: lyon::lyon_tessellation::TessellationError) -> GameError {
        let errstr = format!(
            "Error while tesselating shape (did you give it an infinity or NaN?): {:?}",
            s
        );
        GameError::LyonError(errstr)
    }
}

impl From<lyon::lyon_tessellation::geometry_builder::GeometryBuilderError> for GameError {
    fn from(s: lyon::lyon_tessellation::geometry_builder::GeometryBuilderError) -> GameError {
        let errstr = format!(
            "Error while building geometry (did you give it too many vertices?): {:?}",
            s
        );
        GameError::LyonError(errstr)
    }
}

impl From<strum::ParseError> for GameError {
    fn from(s: strum::ParseError) -> GameError {
        let errstr = format!("Strum parse error: {}", s);
        GameError::ParseError(errstr)
    }
}

impl From<cpal::DefaultStreamConfigError> for GameError {
    fn from(s: cpal::DefaultStreamConfigError) -> GameError {
        let errstr = format!("Default stream config error: {}", s);
        GameError::AudioError(errstr)
    }
}

impl From<cpal::PlayStreamError> for GameError {
    fn from(s: cpal::PlayStreamError) -> GameError {
        let errstr = format!("Play stream error: {}", s);
        GameError::AudioError(errstr)
    }
}

impl From<cpal::BuildStreamError> for GameError {
    fn from(s: cpal::BuildStreamError) -> GameError {
        let errstr = format!("Build stream error: {}", s);
        GameError::AudioError(errstr)
    }
}

impl<T> From<PoisonError<T>> for GameError {
    fn from(s: PoisonError<T>) -> GameError {
        let errstr = format!("Poison error: {}", s);
        GameError::EventLoopError(errstr)
    }
}

impl<T> From<SendError<T>> for GameError {
    fn from(s: SendError<T>) -> GameError {
        let errstr = format!("Send error: {}", s);
        GameError::EventLoopError(errstr)
    }
}
