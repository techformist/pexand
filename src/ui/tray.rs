use crossbeam_channel::Sender;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub struct SystemTray {
    _tray_icon: TrayIcon,
}

impl SystemTray {
    pub fn new(
        ui_tx: Sender<crate::ui::UiExternalMessage>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Rasterize a tiny monochrome mark so the tray has a reliable icon without external assets
        let icon = create_tray_icon()?;

        // Create tray menu
        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Open (Ctrl+Shift+Alt+P)", true, None);
        let quit_item = MenuItem::new("Exit", true, None);

        let show_id = show_item.id().clone();
        let quit_id = quit_item.id().clone();

        tray_menu.append(&show_item)?;
        tray_menu.append(&quit_item)?;

        // Build tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Pexand - Text Expander\nCtrl+Shift+Alt+P to open\nLightweight text expander in the tray")
            .with_icon(icon)
            .build()?;

        // Create channel for menu events
        let menu_channel = MenuEvent::receiver();

        // Create channel for tray icon click events (left/double-click)
        let icon_channel = TrayIconEvent::receiver();

        // Spawn thread to handle menu events
        {
            let ui_tx = ui_tx.clone();
            std::thread::spawn(move || loop {
                if let Ok(event) = menu_channel.recv() {
                    let item_id = event.id;
                    if item_id == show_id {
                        let _ = ui_tx.send(crate::ui::UiExternalMessage::Show);
                    } else if item_id == quit_id {
                        let _ = ui_tx.send(crate::ui::UiExternalMessage::Exit);
                        break;
                    }
                }
            });
        }

        // Spawn thread to handle tray icon click events
        {
            let ui_tx = ui_tx.clone();
            std::thread::spawn(move || loop {
                if let Ok(event) = icon_channel.recv() {
                    match event {
                        TrayIconEvent::Click { .. } => {
                            let _ = ui_tx.send(crate::ui::UiExternalMessage::Show);
                        }
                        _ => {}
                    }
                }
            });
        }

        Ok(Self {
            _tray_icon: tray_icon,
        })
    }
}

fn create_tray_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    const SIZE: u32 = 32;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize]; // RGBA buffer

    // Helper to set pixel at (x, y) with RGBA values
    let set_pixel = |pixels: &mut [u8], x: u32, y: u32, r: u8, g: u8, b: u8, a: u8| {
        let offset = ((y * SIZE + x) * 4) as usize;
        pixels[offset] = r;
        pixels[offset + 1] = g;
        pixels[offset + 2] = b;
        pixels[offset + 3] = a;
    };

    // Colors - white background with accent arrows for visibility
    let accent_r = 102u8;
    let accent_g = 115u8;
    let accent_b = 242u8;
    let accent_a = 255u8;
    let bg_r = 255u8;
    let bg_g = 255u8;
    let bg_b = 255u8;
    let bg_a = 220u8;

    // Draw circular background for better visibility in system tray
    let center = SIZE / 2;
    let radius = 14;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = (x as i32 - center as i32).abs();
            let dy = (y as i32 - center as i32).abs();
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= (radius * radius) {
                set_pixel(&mut pixels, x, y, bg_r, bg_g, bg_b, bg_a);
            }
        }
    }

    // Draw thicker expand icon - 4 corner arrows pointing outward
    // Top-left arrow (pointing to top-left) - thicker
    for i in 0..7 {
        set_pixel(
            &mut pixels,
            7 + i,
            7,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            7 + i,
            8,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            7,
            7 + i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            8,
            7 + i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
    }

    // Top-right arrow (pointing to top-right) - thicker
    for i in 0..7 {
        set_pixel(
            &mut pixels,
            SIZE - 8 - i,
            7,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            SIZE - 8 - i,
            8,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            SIZE - 8,
            7 + i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            SIZE - 9,
            7 + i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
    }

    // Bottom-left arrow (pointing to bottom-left) - thicker
    for i in 0..7 {
        set_pixel(
            &mut pixels,
            7 + i,
            SIZE - 8,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            7 + i,
            SIZE - 9,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            7,
            SIZE - 8 - i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            8,
            SIZE - 8 - i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
    }

    // Bottom-right arrow (pointing to bottom-right) - thicker
    for i in 0..7 {
        set_pixel(
            &mut pixels,
            SIZE - 8 - i,
            SIZE - 8,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            SIZE - 8 - i,
            SIZE - 9,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            SIZE - 8,
            SIZE - 8 - i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
        set_pixel(
            &mut pixels,
            SIZE - 9,
            SIZE - 8 - i,
            accent_r,
            accent_g,
            accent_b,
            accent_a,
        );
    }

    // Draw center square with thicker borders
    for y in 14..19 {
        for x in 14..19 {
            if y == 14 || y == 18 || x == 14 || x == 18 {
                set_pixel(&mut pixels, x, y, accent_r, accent_g, accent_b, accent_a);
            }
        }
    }

    Ok(tray_icon::Icon::from_rgba(pixels, SIZE, SIZE)?)
}
