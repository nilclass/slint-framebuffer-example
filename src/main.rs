use linuxfb::{Framebuffer, set_terminal_mode, TerminalMode};
use std::rc::Rc;
use std::time::Duration;
use slint::{
    PhysicalSize,
    platform::{
        Platform,
        software_renderer::{
            MinimalSoftwareWindow,
            Rgb565Pixel,
            RepaintBufferType,
        },
    },
};

slint::include_modules!();

struct FramebufferPlatform {
    window: Rc<MinimalSoftwareWindow>,
    fb: Framebuffer,
    stride: usize,
}

impl FramebufferPlatform {
    fn new(fb: Framebuffer) -> Self {
        let size = fb.get_size();
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        window.set_size(PhysicalSize::new(size.0, size.1));
        Self {
            window,
            fb,
            stride: size.0 as usize,
        }
    }
}

impl Platform for FramebufferPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        loop {
            slint::platform::update_timers_and_animations();

            self.window.draw_if_needed(|renderer| {
                let mut frame = self.fb.map().unwrap();
                let (_, pixels, _) = unsafe { frame.align_to_mut::<Rgb565Pixel>() };
                renderer.render(pixels, self.stride);
            });

            if !self.window.has_active_animations() {
                std::thread::sleep(slint::platform::duration_until_next_timer_update().unwrap_or(Duration::from_secs(1)));
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // TODO: adjust these values to match your system:

    // Path of the current TTY. Used to switch the terminal to graphics mode
    // and back to text mode.
    let tty_path = "/dev/tty1";
    // Path to the framebuffer device. Normally this is fb0.
    // I'm using a `fbtft` based display on the RaspberryPi, which shows up
    // as fb1 (fb0 is raspi's builtin graphics card).
    let fb_path = "/dev/fb1";

    // Switch back to text mode when terminating
    ctrlc::set_handler(move || {
        let tty = std::fs::File::open(tty_path).unwrap();
        set_terminal_mode(&tty, TerminalMode::Text).expect("switch to text mode");
        std::process::exit(1);
    }).expect("install signal handlers");

    // Instruct slint to use the FramebufferPlatform
    slint::platform::set_platform(Box::new(
        FramebufferPlatform::new(
            Framebuffer::new(fb_path).expect("open framebuffer")
        )
    )).expect("set platform");

    // Switch terminal to graphics mode
    let tty = std::fs::File::open(tty_path).expect("open TTY");
    set_terminal_mode(&tty, TerminalMode::Graphics).expect("switch to graphics mode");
    drop(tty);

    let ui = AppWindow::new()?;

    let ui_handle = ui.as_weak();
    ui.on_request_increase_value(move || {
        let ui = ui_handle.unwrap();
        ui.set_counter(ui.get_counter() + 1);
    });

    ui.run()
}
