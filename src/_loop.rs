
use winit::{event::{ Event, WindowEvent }, event_loop::{ EventLoopWindowTarget, ControlFlow }};

fn hadron_loop() -> Result<(), Box<dyn std::error::Error>> {
    const WINDOW_DIMENSIONS: (i32, i32) = (800, 600);
    
    let eventloop = winit::event_loop::EventLoop::new();
    let window_inner_size = winit::dpi::LogicalSize::new(WINDOW_DIMENSIONS.0, WINDOW_DIMENSIONS.1);
    let window = winit::window::WindowBuilder::new()
        .with_min_inner_size(window_inner_size)
        .with_max_inner_size(window_inner_size).build(&eventloop)?;
    
    let window = Rc::new(window);
    
    let mut app: App = App::new(window.clone());

    let event_handler = move |event: Event<()>, event_loop: &EventLoopWindowTarget<()>, control_flow: &mut ControlFlow| {
        let mut result = EventResult::Ok;
        
        result = match event {
            Event::NewEvents(start) => {
                match start {
                    winit::event::StartCause::ResumeTimeReached { start, requested_resume } => app.dispatch_window_event(window::Event::StartResume(start, requested_resume)),
                    winit::event::StartCause::WaitCancelled { start, requested_resume } => app.dispatch_window_event(window::Event::StartWaitCancelled(start, requested_resume)),
                    winit::event::StartCause::Poll => app.dispatch_window_event(window::Event::StartPolled),
                    winit::event::StartCause::Init => app.dispatch_window_event(window::Event::StartInit),
                }
            },
            Event::WindowEvent{ window_id, event } => {
                match event {
                    WindowEvent::Resized(size) => app.dispatch_window_event(window::Event::Resized(size)),
                    WindowEvent::Moved(position) => app.dispatch_window_event(window::Event::Moved(position)),
                    WindowEvent::CloseRequested => app.dispatch_window_event(window::Event::CloseRequested),
                    WindowEvent::Destroyed => app.dispatch_window_event(window::Event::Destroyed),
                    WindowEvent::DroppedFile(path) => app.dispatch_window_event(window::Event::DroppedFile(path)),
                    WindowEvent::HoveredFile(path) => app.dispatch_window_event(window::Event::HoveredFile(path)),
                    WindowEvent::HoveredFileCancelled => app.dispatch_window_event(window::Event::HoveredFileCancelled()),
                    WindowEvent::ReceivedCharacter(c) => app.dispatch_window_event(window::Event::ReceivedCharacter(c)),
                    WindowEvent::Focused(focused) => app.dispatch_window_event(window::Event::Focused(focused)),
                    WindowEvent::KeyboardInput { device_id, input, is_synthetic } => app.dispatch_window_event(window::Event::KeyboardInput(device_id, input, is_synthetic)),
                    WindowEvent::ModifiersChanged(modifiers_state) => app.dispatch_window_event(window::Event::ModifiersChanged(modifiers_state)),
                    WindowEvent::Ime(ime) => app.dispatch_window_event(window::Event::Ime(ime)),
                    WindowEvent::CursorMoved { device_id, position, ..} => app.dispatch_window_event(window::Event::CursorMoved(device_id, position)),
                    WindowEvent::CursorEntered { device_id } => app.dispatch_window_event(window::Event::CursorEntered(device_id)),
                    WindowEvent::CursorLeft { device_id } => app.dispatch_window_event(window::Event::CursorLeft(device_id)),
                    WindowEvent::MouseWheel { device_id, delta, phase, ..} => app.dispatch_window_event(window::Event::MouseWheel(device_id, delta, phase)),
                    WindowEvent::MouseInput { device_id, state, button, ..} => app.dispatch_window_event(window::Event::MouseInput(device_id, state, button)),
                    WindowEvent::TouchpadPressure { device_id, pressure, stage } => app.dispatch_window_event(window::Event::TouchPadPressure(device_id, pressure, stage)),
                    WindowEvent::AxisMotion { device_id, axis, value } => app.dispatch_window_event(window::Event::AxisMotion(device_id, axis, value)),
                    WindowEvent::Touch(touch) => app.dispatch_window_event(window::Event::Touch(touch)),
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => app.dispatch_window_event(window::Event::ScaleFactorChanged(scale_factor, new_inner_size)),
                    WindowEvent::ThemeChanged(theme) => app.dispatch_window_event(window::Event::ThemeChanged(theme)),
                    WindowEvent::Occluded(occluded) => app.dispatch_window_event(window::Event::Occluded(occluded)),
                }
            },
            Event::DeviceEvent { device_id, event } => {
                match event {
                    winit::event::DeviceEvent::Added => app.dispatch_window_event(window::Event::DeviceAdded),
                    winit::event::DeviceEvent::Removed => app.dispatch_window_event(window::Event::DeviceRemoved),
                    winit::event::DeviceEvent::MouseMotion { delta } => app.dispatch_window_event(window::Event::DeviceMouseMotion(delta)),
                    winit::event::DeviceEvent::MouseWheel { delta } => app.dispatch_window_event(window::Event::DeviceMouseWheel(delta)),
                    winit::event::DeviceEvent::Motion { axis, value } => app.dispatch_window_event(window::Event::DeviceMotion(axis, value)),
                    winit::event::DeviceEvent::Button { button, state } => app.dispatch_window_event(window::Event::DeviceButton(button, state)),
                    winit::event::DeviceEvent::Key(key) => app.dispatch_window_event(window::Event::DeviceKey(key)),
                    winit::event::DeviceEvent::Text { codepoint } => app.dispatch_window_event(window::Event::DeviceText(codepoint)),
                }
            },
            Event::RedrawRequested(window_id) => app.dispatch_window_event(window::Event::Redraw),
            Event::MainEventsCleared => app.dispatch_window_event(window::Event::MainEventsCleared),
            Event::Suspended => app.dispatch_window_event(window::Event::Suspended),
            Event::Resumed => app.dispatch_window_event(window::Event::Resumed),
            Event::RedrawEventsCleared => app.dispatch_window_event(window::Event::RedrawEventsCleared),
            Event::LoopDestroyed => app.dispatch_window_event(window::Event::LoopDestroyed),

            /* Special event used to extend functionality */
            Event::UserEvent(data) => app.dispatch_window_event(window::Event::ExtensionEvent(data)),
        };
        
        // Facilitates App -> Winit communication
        match result {
            EventResult::Ok => { /* All's cool in coolsville */ },
            EventResult::NotImplemented => { /* Handle not implemented events */ },
            EventResult::RedrawRequest => window.request_redraw(),
            EventResult::GraphicsError(error) => panic!("{}", error)
        }
    };
    eventloop.run(event_handler);
}
