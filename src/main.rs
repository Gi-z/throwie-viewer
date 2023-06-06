use std::borrow::{Borrow, BorrowMut};
use minifb::{Window, WindowOptions};
use plotters::prelude::*;
use plotters_bitmap::bitmap_pixel::BGRXPixel;
use plotters_bitmap::BitMapBackend;
use std::error::Error;
use colorous::Gradient;
use std::time::Instant;

mod csi;
mod realtime_heatmap;

// const W: usize = 1000;
// const H: usize = 1000;
const W: usize = 64;
const H: usize = 1000;

const COLOR_SCALE: Gradient = colorous::TURBO;

struct BufferWrapper(Vec<u32>);
impl Borrow<[u8]> for BufferWrapper {
    fn borrow(&self) -> &[u8] {
        // Safe for alignment: align_of(u8) <= align_of(u32)
        // Safe for cast: u32 can be thought of as being transparent over [u8; 4]
        unsafe {
            std::slice::from_raw_parts(
                self.0.as_ptr() as *const u8,
                self.0.len() * 4
            )
        }
    }
}
impl BorrowMut<[u8]> for BufferWrapper {
    fn borrow_mut(&mut self) -> &mut [u8] {
        // Safe for alignment: align_of(u8) <= align_of(u32)
        // Safe for cast: u32 can be thought of as being transparent over [u8; 4]
        unsafe {
            std::slice::from_raw_parts_mut(
                self.0.as_mut_ptr() as *mut u8,
                self.0.len() * 4
            )
        }
    }
}
impl Borrow<[u32]> for BufferWrapper {
    fn borrow(&self) -> &[u32] {
        self.0.as_slice()
    }
}
impl BorrowMut<[u32]> for BufferWrapper {
    fn borrow_mut(&mut self) -> &mut [u32] {
        self.0.as_mut_slice()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Open CSI UDP port.
    let socket = csi::open_csi_socket();
    println!("Successfully bound port {}.", csi::UDP_SERVER_PORT);

    let mut matrix: [[f32; 64]; realtime_heatmap::WINDOW_SIZE] = [[0_f32; 64]; realtime_heatmap::WINDOW_SIZE];
    let mut maxval: f32 = 0.0;

    let mut buf = BufferWrapper(vec![0u32; W * H]);

    let mut window = Window::new(
        "CSI Heatmap",
        W,
        H,
        // WindowOptions::default(),
        WindowOptions {
            borderless: false,
            title: true,
            resize: true,
            scale: minifb::Scale::X1,
            scale_mode: minifb::ScaleMode::Stretch,
            topmost: false,
            transparency: false,
            none: false
        }
    )?;

    {
        let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
            buf.borrow_mut(),
            (W as u32, H as u32),
        )?
        .into_drawing_area();
        root.fill(&BLACK)?;
        root.present()?;
    };

    loop {
        // let now = Instant::now();

        let recv_result = csi::recv_message(&socket);
        let msg = match recv_result {
            Ok(m) => m,
            Err(_) => continue
        };

        let measurement = csi::get_csi_measurement(&msg);
        let src_mac = format!("0x{:X}", msg.src_mac.clone().unwrap()[5]);

        if src_mac == "0x62" {
            (matrix, maxval) = realtime_heatmap::update_matrix(matrix, maxval, &measurement);

            {
                let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
                    buf.borrow_mut(),
                    (W as u32, H as u32),
                )?
                .into_drawing_area();

                let cells = root.split_evenly((realtime_heatmap::WINDOW_SIZE, 64));

                for (cell, csi) in std::iter::zip(cells.iter(), matrix.into_iter().flatten()) {
                    let mag_scaled = csi.sqrt() / maxval.sqrt();
                    let color = COLOR_SCALE.eval_continuous(mag_scaled as f64);
                    cell.fill(&RGBColor(color.r, color.g, color.b))?;
                }

                root.present()?;
            }

            // Manually updating the u32 framebuffer if you wanna be weird like that
            // {
            //     let mut b: &mut [u32] = buf.borrow_mut();
            //     for (i, csi) in matrix.into_iter().flatten().enumerate() {
            //         let mag_scaled = csi.sqrt() / maxval.sqrt();
            //         let color = COLOR_SCALE.eval_continuous(mag_scaled as f64);
                    
            //         let mut rgb: u32 = 0;
            //         rgb += color.r as u32;
            //         rgb = (rgb << 8) + color.g as u32;
            //         rgb = (rgb << 8) + color.b as u32;
            //         b[i] = rgb;
            //     }
            // }
            
            window.update_with_buffer(buf.borrow(), W, H)?;

            // let after = Instant::now();
            // println!("Total Data/Frame Processing time: {:.2?}", after - now);
        }
    }
}