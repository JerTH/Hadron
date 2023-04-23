use std::{rc::Rc, time::{Instant, Duration}, borrow::BorrowMut};
use winit::{event::{ Event, WindowEvent }, event_loop::{ EventLoopWindowTarget, ControlFlow }};

use crate::{graphics::vulkangfx::TVulkanGraphics, debug::dump_backtrace};
use crate::graphics::vulkan_experimental::VulkanResult;
use crate::app::window::EventErrorResult;
use crate::graphics::vulkan_experimental::VulkanGraphics as VulkanExperimental;

pub struct App {
    eventloop: Option<winit::event_loop::EventLoop<()>>,
    window: Rc<winit::window::Window>,
    graphics: GraphicsImpl,
    counters: AppCounters,
}

pub(crate) enum GraphicsImpl {
    None,
    VulkanGraphics(TVulkanGraphics),
    VulkanExperimental(VulkanExperimental),
}

/// App-centric events
pub(crate) enum AppEvent { }

pub(crate) enum AppEventResult {
    Ok,
    NotImplemented,
    RedrawRequest,
    GraphicsError(Box<dyn std::error::Error>),
}

struct AppCounters {
    redraws: u64,
    frame_begin: Option<Instant>,
    frame_end: Option<Instant>,
    frame_average: Option<Duration>,
}

/// Anything related to the window/winit
pub(crate) mod window {
    /// Window-centric events
    pub(crate) enum WindowEvent<'a> {
        // App events
        Redraw,

        // Winit events
        Resized(winit::dpi::PhysicalSize<u32>),
        Moved(winit::dpi::PhysicalPosition<i32>),
        CloseRequested,
        Destroyed,
        DroppedFile(std::path::PathBuf),
        HoveredFile(std::path::PathBuf),
        HoveredFileCancelled(),
        ReceivedCharacter(char),
        Focused(bool),
        KeyboardInput(winit::event::DeviceId, winit::event::KeyboardInput, bool),
        ModifiersChanged(winit::event::ModifiersState),
        Ime(winit::event::Ime),
        CursorMoved(winit::event::DeviceId, winit::dpi::PhysicalPosition<f64>),
        CursorEntered(winit::event::DeviceId),
        CursorLeft(winit::event::DeviceId),
        MouseWheel(winit::event::DeviceId, winit::event::MouseScrollDelta, winit::event::TouchPhase),
        MouseInput(winit::event::DeviceId, winit::event::ElementState, winit::event::MouseButton),
        TouchPadPressure(winit::event::DeviceId, f32, i64),
        AxisMotion(winit::event::DeviceId, u32, f64),
        Touch(winit::event::Touch),
        ScaleFactorChanged(f64, &'a mut winit::dpi::PhysicalSize<u32>),
        ThemeChanged(winit::window::Theme),
        Occluded(bool),

        // Winit device events 
        DeviceAdded,
        DeviceRemoved,
        DeviceMouseMotion((f64, f64)),
        DeviceMouseWheel(winit::event::MouseScrollDelta),
        DeviceMotion(u32, f64),
        DeviceButton(u32, winit::event::ElementState),
        DeviceKey(winit::event::KeyboardInput),
        DeviceText(char),

        // Winit loop start events
        StartResume(std::time::Instant, std::time::Instant),
        StartWaitCancelled(std::time::Instant, Option<std::time::Instant>),
        StartPolled,
        StartInit,

        MainEventsCleared,
        ExtensionEvent(()),
        Suspended,
        RedrawEventsCleared,
        Resumed,
        LoopDestroyed,
    }

    pub(crate) enum EventErrorResult {
        VulkanError(ash::vk::Result),
    }
}

impl App {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // The app is the consumer of hadron, the hadron should request
        // a config from the app with a callback, if it doesn't receive one
        // then it should try to load one from disk. If there isn't one to load
        // then it should use a default configuration baked into the executable

        const WINDOW_DIMENSIONS: (i32, i32) = (800, 600);
        
        let eventloop = winit::event_loop::EventLoop::new();

        let window_inner_size = winit::dpi::LogicalSize::new(WINDOW_DIMENSIONS.0, WINDOW_DIMENSIONS.1);
        
        let window = winit::window::WindowBuilder::new()
            .with_min_inner_size(window_inner_size)
            .with_max_inner_size(window_inner_size).build(&eventloop)?;
        
        let window = Rc::new(window);
        
        let vulkan_graphics = VulkanExperimental::new(window.clone()).unwrap();
        let graphics = GraphicsImpl::VulkanExperimental(vulkan_graphics);
        
        Ok(App {
            eventloop: Some(eventloop),
            window,
            graphics,
            counters: AppCounters::zero(),
        })
    }

    pub(crate) fn dispatch_window_event(&mut self, event: window::WindowEvent) -> AppEventResult {
        let result = match event {
            window::WindowEvent::Redraw => self.event_redraw(),
            window::WindowEvent::Resized(_) => self.event_resized(),
            window::WindowEvent::Moved(_) => AppEventResult::NotImplemented,
            window::WindowEvent::CloseRequested => AppEventResult::NotImplemented,
            window::WindowEvent::Destroyed => AppEventResult::NotImplemented,
            window::WindowEvent::DroppedFile(_) => AppEventResult::NotImplemented,
            window::WindowEvent::HoveredFile(_) => AppEventResult::NotImplemented,
            window::WindowEvent::HoveredFileCancelled() => AppEventResult::NotImplemented,
            window::WindowEvent::ReceivedCharacter(_) => AppEventResult::NotImplemented,
            window::WindowEvent::Focused(_) => self.event_focused(),
            window::WindowEvent::KeyboardInput(_, _, _) => AppEventResult::NotImplemented,
            window::WindowEvent::ModifiersChanged(_) => AppEventResult::NotImplemented,
            window::WindowEvent::Ime(_) => AppEventResult::NotImplemented,
            window::WindowEvent::CursorMoved(_, _) => AppEventResult::NotImplemented,
            window::WindowEvent::CursorEntered(_) => self.event_cursor_entered(),
            window::WindowEvent::CursorLeft(_) => self.event_cursor_left(),
            window::WindowEvent::MouseWheel(_, _, _) => AppEventResult::NotImplemented,
            window::WindowEvent::MouseInput(_, _, _) => AppEventResult::NotImplemented,
            window::WindowEvent::TouchPadPressure(_, _, _) => AppEventResult::NotImplemented,
            window::WindowEvent::AxisMotion(_, _, _) => AppEventResult::NotImplemented,
            window::WindowEvent::Touch(_) => AppEventResult::NotImplemented,
            window::WindowEvent::ScaleFactorChanged(_, _) => AppEventResult::NotImplemented,
            window::WindowEvent::ThemeChanged(_) => AppEventResult::NotImplemented,
            window::WindowEvent::Occluded(_) => AppEventResult::NotImplemented,
            window::WindowEvent::MainEventsCleared => self.event_main_events_cleared(),
            
            window::WindowEvent::DeviceAdded => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceRemoved => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceMouseMotion(_) => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceMouseWheel(_) => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceMotion(_, _) => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceButton(_, _) => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceKey(_) => AppEventResult::NotImplemented,
            window::WindowEvent::DeviceText(_) => AppEventResult::NotImplemented,

            window::WindowEvent::StartResume(_, _) => self.event_start_resume(),
            window::WindowEvent::StartWaitCancelled(_, _) => self.event_start_wait_cancelled(),
            window::WindowEvent::StartPolled => self.event_start_polled(),
            window::WindowEvent::StartInit => self.event_start_init(),
            
            window::WindowEvent::Suspended => AppEventResult::NotImplemented,
            window::WindowEvent::RedrawEventsCleared => self.event_redraw_events_cleared(),
            window::WindowEvent::Resumed => AppEventResult::NotImplemented,
            window::WindowEvent::LoopDestroyed => AppEventResult::NotImplemented,
            window::WindowEvent::ExtensionEvent(_) => AppEventResult::NotImplemented,
        };
        return result;
    }
    
    fn event_redraw(&mut self) -> AppEventResult {
        match self.graphics.borrow_mut() {
            GraphicsImpl::None => {
                AppEventResult::Ok
            },
            GraphicsImpl::VulkanGraphics(gfx) => {
                gfx.wait_for_fences();
                let image_index = gfx.next_image();
                gfx.reset_fences();
                gfx.submit_commandbuffer(image_index);
                gfx.swapchain().present(image_index, gfx.graphics_device().graphics_queue());

                self.counters.increment_redraw_count();
                AppEventResult::Ok
            },
            GraphicsImpl::VulkanExperimental(gfx) => {
                AppEventResult::NotImplemented
            },
        }
    }

    fn event_resized(&self) -> AppEventResult {
        AppEventResult::Ok
    }

    fn event_focused(&self) -> AppEventResult {
        AppEventResult::Ok
    }

    fn event_cursor_entered(&self) -> AppEventResult {
        AppEventResult::Ok
    }

    fn event_cursor_left(&self) -> AppEventResult {
        AppEventResult::Ok
    }

    fn event_main_events_cleared(&self) -> AppEventResult {
        AppEventResult::RedrawRequest
    }

    fn event_start_resume(&mut self) -> AppEventResult {
        self.begin_frame();
        AppEventResult::Ok
    }

    fn event_start_wait_cancelled(&mut self) -> AppEventResult {
        self.begin_frame();
        AppEventResult::Ok
    }

    fn event_start_polled(&mut self) -> AppEventResult {
        self.begin_frame();
        AppEventResult::Ok
    }
    
    fn event_start_init(&mut self) -> AppEventResult {
        println!("Start init");
        self.begin_frame();
        
        match VulkanExperimental::new(self.window.clone()) {
            Ok(graphics) => {
                self.graphics = GraphicsImpl::VulkanExperimental(graphics);
                AppEventResult::Ok
            },
            Err(result) => {
                match result {
                    VulkanResult::Success => todo!(),
                    VulkanResult::NotReady => todo!(),
                    VulkanResult::Timeout => todo!(),
                    VulkanResult::EventSet => todo!(),
                    VulkanResult::EventReset => todo!(),
                    VulkanResult::Incomplete => todo!(),
                    VulkanResult::Error(error) => AppEventResult::GraphicsError(Box::new(error)),
                }
            },
        }
        
        //match TVulkanGraphics::init(self.window.clone()) {
        //    Ok(graphics) => {
        //        self.graphics = GraphicsImpl::VulkanGraphics(graphics);
        //        AppEventResult::Ok
        //    },
        //    Err(err) => {
        //        AppEventResult::Error(EventErrorResult::VulkanError(err))
        //    },
        //}

    }

    fn begin_frame(&mut self) {
        self.counters.begin_frame_clock();
    }

    fn end_frame(&mut self) -> Option<Duration> {
        self.counters.end_frame_clock()
    }
    
    
    fn event_redraw_events_cleared(&mut self) -> AppEventResult {
        match self.end_frame() {
            Some(_) => {
                match self.counters.average_frame_duration() {
                    Some(average_frame_time) => {
                        if self.counters.redraws % 5 == 0 {
                            println!("fps: {:.1}, frame: {}", 1.0 / average_frame_time.as_secs_f64(), self.counters.redraws);
                        }
                    },
                    None => {
                        
                        /* We don't have an average yet */
                    },
                }
            },
            None => { /* First frame condition */ },
        }

        AppEventResult::Ok
    }

    pub fn run(self) -> ! {
        self.main_loop()
    }
    
    /// The main app loop
    fn main_loop(mut self) -> ! {
        // The eventloop is packaged inside of the App for convenience, however, it has to be separated out before we run it, as it self-references the App
        let eventloop = self.eventloop.take().expect("No event loop");
        
        // Setup out event handler for the eventloop
        let event_handler = move |event: Event<()>, event_loop: &EventLoopWindowTarget<()>, control_flow: &mut ControlFlow| {
            let mut result = AppEventResult::Ok;
            let app = &mut self;

            result = match event {
                Event::NewEvents(start) => {
                    match start {
                        winit::event::StartCause::ResumeTimeReached { start, requested_resume } => self.dispatch_window_event(window::WindowEvent::StartResume(start, requested_resume)),
                        winit::event::StartCause::WaitCancelled { start, requested_resume } => self.dispatch_window_event(window::WindowEvent::StartWaitCancelled(start, requested_resume)),
                        winit::event::StartCause::Poll => self.dispatch_window_event(window::WindowEvent::StartPolled),
                        winit::event::StartCause::Init => self.dispatch_window_event(window::WindowEvent::StartInit),
                    }
                },
                Event::WindowEvent{ window_id, event } => {
                    match event {
                        WindowEvent::Resized(size) => self.dispatch_window_event(window::WindowEvent::Resized(size)),
                        WindowEvent::Moved(position) => self.dispatch_window_event(window::WindowEvent::Moved(position)),
                        WindowEvent::CloseRequested => self.dispatch_window_event(window::WindowEvent::CloseRequested),
                        WindowEvent::Destroyed => self.dispatch_window_event(window::WindowEvent::Destroyed),
                        WindowEvent::DroppedFile(path) => self.dispatch_window_event(window::WindowEvent::DroppedFile(path)),
                        WindowEvent::HoveredFile(path) => self.dispatch_window_event(window::WindowEvent::HoveredFile(path)),
                        WindowEvent::HoveredFileCancelled => self.dispatch_window_event(window::WindowEvent::HoveredFileCancelled()),
                        WindowEvent::ReceivedCharacter(c) => self.dispatch_window_event(window::WindowEvent::ReceivedCharacter(c)),
                        WindowEvent::Focused(focused) => self.dispatch_window_event(window::WindowEvent::Focused(focused)),
                        WindowEvent::KeyboardInput { device_id, input, is_synthetic } => self.dispatch_window_event(window::WindowEvent::KeyboardInput(device_id, input, is_synthetic)),
                        WindowEvent::ModifiersChanged(modifiers_state) => self.dispatch_window_event(window::WindowEvent::ModifiersChanged(modifiers_state)),
                        WindowEvent::Ime(ime) => self.dispatch_window_event(window::WindowEvent::Ime(ime)),
                        WindowEvent::CursorMoved { device_id, position, ..} => self.dispatch_window_event(window::WindowEvent::CursorMoved(device_id, position)),
                        WindowEvent::CursorEntered { device_id } => self.dispatch_window_event(window::WindowEvent::CursorEntered(device_id)),
                        WindowEvent::CursorLeft { device_id } => self.dispatch_window_event(window::WindowEvent::CursorLeft(device_id)),
                        WindowEvent::MouseWheel { device_id, delta, phase, ..} => self.dispatch_window_event(window::WindowEvent::MouseWheel(device_id, delta, phase)),
                        WindowEvent::MouseInput { device_id, state, button, ..} => self.dispatch_window_event(window::WindowEvent::MouseInput(device_id, state, button)),
                        WindowEvent::TouchpadPressure { device_id, pressure, stage } => self.dispatch_window_event(window::WindowEvent::TouchPadPressure(device_id, pressure, stage)),
                        WindowEvent::AxisMotion { device_id, axis, value } => self.dispatch_window_event(window::WindowEvent::AxisMotion(device_id, axis, value)),
                        WindowEvent::Touch(touch) => self.dispatch_window_event(window::WindowEvent::Touch(touch)),
                        WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => self.dispatch_window_event(window::WindowEvent::ScaleFactorChanged(scale_factor, new_inner_size)),
                        WindowEvent::ThemeChanged(theme) => self.dispatch_window_event(window::WindowEvent::ThemeChanged(theme)),
                        WindowEvent::Occluded(occluded) => self.dispatch_window_event(window::WindowEvent::Occluded(occluded)),
                    }
                },
                Event::DeviceEvent { device_id, event } => {
                    match event {
                        winit::event::DeviceEvent::Added => self.dispatch_window_event(window::WindowEvent::DeviceAdded),
                        winit::event::DeviceEvent::Removed => self.dispatch_window_event(window::WindowEvent::DeviceRemoved),
                        winit::event::DeviceEvent::MouseMotion { delta } => self.dispatch_window_event(window::WindowEvent::DeviceMouseMotion(delta)),
                        winit::event::DeviceEvent::MouseWheel { delta } => self.dispatch_window_event(window::WindowEvent::DeviceMouseWheel(delta)),
                        winit::event::DeviceEvent::Motion { axis, value } => self.dispatch_window_event(window::WindowEvent::DeviceMotion(axis, value)),
                        winit::event::DeviceEvent::Button { button, state } => self.dispatch_window_event(window::WindowEvent::DeviceButton(button, state)),
                        winit::event::DeviceEvent::Key(key) => self.dispatch_window_event(window::WindowEvent::DeviceKey(key)),
                        winit::event::DeviceEvent::Text { codepoint } => self.dispatch_window_event(window::WindowEvent::DeviceText(codepoint)),
                    }
                },
                Event::RedrawRequested(window_id) => self.dispatch_window_event(window::WindowEvent::Redraw),
                Event::MainEventsCleared => self.dispatch_window_event(window::WindowEvent::MainEventsCleared),
                Event::Suspended => self.dispatch_window_event(window::WindowEvent::Suspended),
                Event::Resumed => self.dispatch_window_event(window::WindowEvent::Resumed),
                Event::RedrawEventsCleared => self.dispatch_window_event(window::WindowEvent::RedrawEventsCleared),
                Event::LoopDestroyed => self.dispatch_window_event(window::WindowEvent::LoopDestroyed),

                /* Special event used to extend functionality */
                Event::UserEvent(data) => self.dispatch_window_event(window::WindowEvent::ExtensionEvent(data)),
            };

            // Facilitates App -> Winit communication
            match result {
                AppEventResult::Ok => { /* All's cool in coolsville */ },
                AppEventResult::NotImplemented => { /* Handle not implemented events */ },
                AppEventResult::RedrawRequest => self.window.request_redraw(),
                AppEventResult::GraphicsError(error) => {
                    dump_backtrace();
                    panic!("{}", error);
                }
            }
        };

        // Executes the event loop. Never returns
        eventloop.run(event_handler);
    }
}

impl From<VulkanResult> for AppEventResult {
    fn from(result: VulkanResult) -> Self {
        match result {
            VulkanResult::Success => AppEventResult::Ok,

            /* These are vulkan implementation results which do not constitute errors, and don't typically need to be handled */
            VulkanResult::NotReady => AppEventResult::Ok,
            VulkanResult::Timeout => AppEventResult::Ok,
            VulkanResult::EventSet => AppEventResult::Ok,
            VulkanResult::EventReset => AppEventResult::Ok,
            VulkanResult::Incomplete => AppEventResult::Ok,

            /* Any actual vulkan errors get boxed into a GraphicsError, these should be handled */
            VulkanResult::Error(error) => AppEventResult::GraphicsError(Box::new(error)),
        }
    }
}

impl AppCounters {
    fn zero() -> Self {
        AppCounters {
            redraws: 0u64,
            frame_begin: None,
            frame_end: None,
            frame_average: None,
        }
    }

    fn increment_redraw_count(&mut self) {
        self.redraws = self.redraws + 1;
    }

    /// Begins a frame clock, if a previous frame was measured, returns the total duration since end_frame_clock() was called
    /// calling this twice in a row without calling end_frame_clock resets the clock and returns `None`
    /// 
    /// Returns `None` if this is the first frame to be measured
    fn begin_frame_clock(&mut self) -> Option<Duration> {
        let now = Instant::now();
        self.frame_begin = Some(now);

        match self.frame_end {
            Some(last_frame_end) => {
                let dur = now.duration_since(last_frame_end);
                return Some(dur)
            },
            None => {
                return None
            },
        }
    }
    
    /// Ends a frame clock and returns the total duration since begin_frame_clock() was called
    /// 
    /// Returns `None` if a frame clock was not yet started
    fn end_frame_clock(&mut self) -> Option<Duration> {
        match self.frame_begin {
            Some(begin) => {
                let now = Instant::now();
                let dur = now.duration_since(begin);

                // New average = old average * (n-1)/n + new value /n
                match self.frame_average {
                    Some(old_average) => {
                        let old_average = old_average.as_secs_f64();
                        let this_frame = dur.as_secs_f64();
                        let smoothing = 0.9f64;
                        let new_average = Duration::from_secs_f64((old_average * smoothing) + (this_frame * (1.0f64 - smoothing)));
                        self.frame_average = Some(new_average);
                    },
                    None => {
                        self.frame_average = Some(dur)
                    },
                }

                self.frame_end = Some(now);
                return Some(dur)
            },
            None => {
                return None
            },
        }
    }

    fn average_frame_duration(&self) -> Option<Duration> {
        self.frame_average
    }
}

#[cfg(test)]
mod test {

}
