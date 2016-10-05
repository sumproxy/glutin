use winit;

use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use api::wayland;
use api::x11;

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;

pub enum Window {
    #[doc(hidden)]
    X(x11::Window),
    #[doc(hidden)]
    Wayland(wayland::Window)
}

pub use winit::platform::{MonitorId, get_available_monitors, get_primary_monitor};

impl Window {
    #[inline]
    pub fn new(
        window: &WindowAttributes, // вот это надо бы убрать
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        _: &PlatformSpecificWindowBuilderAttributes, // и это, наверное, тоже убрать
        ozkriff_window: &winit::Window,
    ) -> Result<Window, CreationError> {
        match ozkriff_window.window {
            winit::platform::Window::X(_) => {
                let opengl = opengl.clone().map_sharing(|w| match w {
                    &Window::X(ref w) => w,
                    _ => panic!()       // TODO: return an error
                });
                x11::Window::new(
                    pf_reqs,
                    &opengl,
                    ozkriff_window,
                ).map(Window::X)
            },
            winit::platform::Window::Wayland(_) => {
                let opengl = opengl.clone().map_sharing(|w| match w {
                    &Window::Wayland(ref w) => w,
                    _ => panic!()       // TODO: return an error
                });
                wayland::Window::new(
                    window,
                    pf_reqs,
                    &opengl,
                    ozkriff_window,
                ).map(Window::Wayland)
            },
        }
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self {
            &Window::X(ref w) => w.make_current(),
            &Window::Wayland(ref w) => w.make_current()
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self {
            &Window::X(ref w) => w.is_current(),
            &Window::Wayland(ref w) => w.is_current()
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self {
            &Window::X(ref w) => w.get_proc_address(addr),
            &Window::Wayland(ref w) => w.get_proc_address(addr)
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self {
            &Window::X(ref w) => w.swap_buffers(),
            &Window::Wayland(ref w) => w.swap_buffers()
        }
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        match self {
            &Window::X(ref w) => w.get_api(),
            &Window::Wayland(ref w) => w.get_api()
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self {
            &Window::X(ref w) => w.get_pixel_format(),
            &Window::Wayland(ref w) => w.get_pixel_format()
        }
    }
}
