
#![allow(dead_code)]
use std::num::NonZeroU32;
use glium::Display;
use glutin::prelude::*;
use glutin::display::GetGlDisplay;
use glutin::surface::WindowSurface;
use raw_window_handle::HasWindowHandle;
use winit::event_loop::ActiveEventLoop;
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::application::ApplicationHandler;

pub mod camera;
pub mod mouse;
pub mod field;

// 800x600

use mouse::Mouse;
//use field::VectorField2D;


pub trait ApplicationContext {
    fn draw_frame(&mut self, _display: &Display<WindowSurface>) { }
    fn new(display: &Display<WindowSurface>) -> Self;
    fn update(&mut self) { }
    fn handle_window_event(&mut self, _event: &glium::winit::event::WindowEvent, _window: &glium::winit::window::Window) { }
    const WINDOW_TITLE:&'static str;
}

pub struct State<T> {
    pub display: glium::Display<WindowSurface>,
    pub window: glium::winit::window::Window,
    pub context: T,
}

struct App<T> {
    state: Option<State<T>>,
    visible: bool,
    close_promptly: bool,
    old_mouse_x: i16,
    old_mouse_y: i16
}

impl<T: ApplicationContext + 'static> ApplicationHandler<()> for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(State::new(event_loop, self.visible));
        if !self.visible && self.close_promptly {
            event_loop.exit();
        }
    }
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.state = None;
    }

    fn window_event(&mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            glium::winit::event::WindowEvent::Resized(new_size) => {
                if let Some(state) = &self.state {
                    state.display.resize(new_size.into());
                }
            },
            glium::winit::event::WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.context.update();
                    state.context.draw_frame(&state.display);
                    if self.close_promptly {
                        event_loop.exit();
                    }
                }
            },
            glium::winit::event::WindowEvent::CloseRequested
            | glium::winit::event::WindowEvent::KeyboardInput { event: glium::winit::event::KeyEvent {
                state: glium::winit::event::ElementState::Pressed,
                logical_key: glium::winit::keyboard::Key::Named(glium::winit::keyboard::NamedKey::Escape),
                ..
            }, ..} => {
                event_loop.exit()
            },
            glium::winit::event::WindowEvent::CursorMoved { position, .. } => {
                let position_x: i16 = position.x as i16;
                let position_y: i16 = position.y as i16;

                Mouse::update_position(position_x, position_y);
        
                let (delta_x, delta_y) = Mouse::get_delta();
                let (x, y) = Mouse::get_position();

                //VectorField2D::onMouseClick(x, y, delta_x, delta_y);
        
                println!("Delta mouse position: {}x{}", delta_x, delta_y);
                println!("Mouse position: {}x{}", x, y);
            },
            glium::winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if state == glium::winit::event::ElementState::Pressed {
                    println!("Mouse clicked: {:?}", button);
                }
            },
            ev => {
                if let Some(state) = &mut self.state {
                    state.context.handle_window_event(&ev, &state.window);
                }
            },
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

impl<T: ApplicationContext + 'static> State<T> {
    pub fn new(
        event_loop: &glium::winit::event_loop::ActiveEventLoop,
        visible: bool,
    ) -> Self {
        let window_attributes = winit::window::Window::default_attributes()
            .with_title(T::WINDOW_TITLE).with_visible(visible);
        let config_template_builder = glutin::config::ConfigTemplateBuilder::new();
        let display_builder = glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder
            .build(event_loop, config_template_builder, |mut configs| {
                configs.next().unwrap()
            })
            .unwrap();
        let window = window.unwrap();

        let window_handle = window.window_handle().expect("couldn't obtain window handle");
        let context_attributes = glutin::context::ContextAttributesBuilder::new().build(Some(window_handle.into()));
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(Some(window_handle.into()));

        let not_current_gl_context = Some(unsafe {
            gl_config.display().create_context(&gl_config, &context_attributes).unwrap_or_else(|_| {
                gl_config.display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
        });

        // Determine our framebuffer size based on the window size, or default to 800x600 if it's invisible
        let (width, height): (u32, u32) = if visible { window.inner_size().into() } else { (800, 600) };
        let attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.into(),
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        // Now we can create our surface, use it to make our context current and finally create our display
        let surface = unsafe { gl_config.display().create_window_surface(&gl_config, &attrs).unwrap() };
        let current_context = not_current_gl_context.unwrap().make_current(&surface).unwrap();
        let display = glium::Display::from_context_surface(current_context, surface).unwrap();

        Self::from_display_window(display, window)
    }

    pub fn from_display_window(
        display: glium::Display<WindowSurface>,
        window: glium::winit::window::Window,
    ) -> Self {
        let context = T::new(&display);
        Self {
            display,
            window,
            context,
        }
    }

    pub fn run_loop() {
        let event_loop = glium::winit::event_loop::EventLoop::builder()
            .build()
            .expect("event loop building");
        let mut app = App::<T> {
            state: None,
            visible: true,
            close_promptly: false,
            old_mouse_x: 0,
            old_mouse_y: 0
        };
        let result = event_loop.run_app(&mut app);
        result.unwrap();
    }

    pub fn run_once(visible: bool) {
        let event_loop = glium::winit::event_loop::EventLoop::builder()
            .build()
            .expect("event loop building");
        let mut app = App::<T> {
            state: None,
            visible,
            close_promptly: true,
            old_mouse_x: 0,
            old_mouse_y: 0
        };
        let result = event_loop.run_app(&mut app);
        result.unwrap();
    }
}
