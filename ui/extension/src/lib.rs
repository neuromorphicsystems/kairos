use wasm_bindgen::prelude::*;

const MAXIMUM_DELTA: u64 = 3600000000;

#[wasm_bindgen]
pub struct Renderer {
    width: u16,
    height: u16,
    ts_and_ons: Vec<f32>,
    evt3_buffer: Vec<u8>,

    // decode state
    t: u64,
    overflows: u32,
    previous_msb_t: u16,
    previous_lsb_t: u16,
    x: u16,
    y: u16,
    on: bool,

    // render state
    offset_t: u64,
}

#[wasm_bindgen]
impl Renderer {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u16, height: u16, evt3_buffer_maximum_length: usize) -> Renderer {
        Renderer {
            width,
            height,
            ts_and_ons: vec![(MAXIMUM_DELTA * 2) as f32; width as usize * height as usize],
            evt3_buffer: vec![0; evt3_buffer_maximum_length],
            t: 0,
            overflows: 0,
            previous_msb_t: 0,
            previous_lsb_t: 0,
            x: 0,
            y: 0,
            on: false,
            offset_t: 0,
        }
    }

    pub fn ts_and_ons_pointer(&self) -> *const u8 {
        self.ts_and_ons.as_ptr() as *const u8
    }
    pub fn ts_and_ons_pointer_byte_length(&self) -> usize {
        self.ts_and_ons.len() * std::mem::size_of::<f32>()
    }

    pub fn evt3_buffer_pointer(&self) -> *const u8 {
        self.evt3_buffer.as_ptr()
    }
    pub fn evt3_buffer_length(&self) -> usize {
        self.evt3_buffer.len()
    }

    pub fn current_t(&self) -> u64 {
        self.t
    }

    pub fn gl_current_t(&self) -> f32 {
        (self.t - self.offset_t) as f32
    }
}

#[wasm_bindgen]
pub fn render(renderer: &mut Renderer, evt3_buffer_length: usize) {
    for index in 34..evt3_buffer_length / 2 {
        let word = u16::from_le_bytes([
            renderer.evt3_buffer[index * 2],
            renderer.evt3_buffer[index * 2 + 1],
        ]);
        match word >> 12 {
            0b0000 => {
                renderer.y = word & 0b11111111111;
            }
            0b0001 => (),
            0b0010 => {
                renderer.x = word & 0b11111111111;
                renderer.on = (word & (1 << 11)) > 0;
                if renderer.x < renderer.width && renderer.y < renderer.height {
                    renderer.ts_and_ons
                        [renderer.x as usize + renderer.y as usize * renderer.width as usize] =
                        if renderer.on {
                            (renderer.t - renderer.offset_t) as f32
                        } else {
                            -((renderer.t - renderer.offset_t) as f32)
                        };
                }
            }
            0b0011 => {
                renderer.x = word & 0b11111111111;
                renderer.on = (word & (1 << 11)) > 0;
            }
            0b0100 => {
                if renderer.x < renderer.width && renderer.y < renderer.height {
                    let set = word & ((1 << std::cmp::min(12, renderer.width - renderer.x)) - 1);
                    let t_and_on = if renderer.on {
                        (renderer.t - renderer.offset_t) as f32
                    } else {
                        -((renderer.t - renderer.offset_t) as f32)
                    };
                    for bit in 0..12 {
                        if (set & (1 << bit)) > 0 {
                            renderer.ts_and_ons[renderer.x as usize
                                + renderer.y as usize * renderer.width as usize] = t_and_on;
                        }
                    }
                    renderer.x = renderer.x.overflowing_add(12).0;
                }
            }
            0b0101 => {
                if renderer.x < renderer.width && renderer.y < renderer.height {
                    let set = word & ((1 << std::cmp::min(8, renderer.width - renderer.x)) - 1);
                    let t_and_on = if renderer.on {
                        (renderer.t - renderer.offset_t) as f32
                    } else {
                        -((renderer.t - renderer.offset_t) as f32)
                    };
                    for bit in 0..8 {
                        if (set & (1 << bit)) > 0 {
                            renderer.ts_and_ons[renderer.x as usize
                                + renderer.y as usize * renderer.width as usize] = t_and_on;
                        }
                    }
                    renderer.x = renderer.x.overflowing_add(8).0;
                }
            }
            0b0110 => {
                let lsb_t = word & 0b111111111111;
                if lsb_t != renderer.previous_lsb_t {
                    renderer.previous_lsb_t = lsb_t;
                    let t = (((renderer.previous_lsb_t as u32)
                        | ((renderer.previous_msb_t as u32) << 12))
                        as u64)
                        | ((renderer.overflows as u64) << 24);
                    if t >= renderer.t {
                        renderer.t = t;
                    }
                }
            }
            0b0111 => (),
            0b1000 => {
                let msb_t = word & 0b111111111111;
                if msb_t != renderer.previous_msb_t {
                    if msb_t > renderer.previous_msb_t {
                        if (msb_t - renderer.previous_msb_t) < (1 << 11) {
                            renderer.previous_lsb_t = 0;
                            renderer.previous_msb_t = msb_t;
                        }
                    } else if (renderer.previous_msb_t - msb_t) > (1 << 11) {
                        renderer.overflows += 1;
                        renderer.previous_lsb_t = 0;
                        renderer.previous_msb_t = msb_t;
                    }
                    let t = (((renderer.previous_lsb_t as u32)
                        | ((renderer.previous_msb_t as u32) << 12))
                        as u64)
                        | ((renderer.overflows as u64) << 24);
                    if t >= renderer.t {
                        renderer.t = t;
                    }
                }
            }
            0b1001 => (),
            0b1010 => (),
            #[allow(clippy::manual_range_patterns)]
            0b1011 | 0b1100 | 0b1101 | 0b1110 | 0b1111 => (),
            _ => (),
        }
    }
    while renderer.t - renderer.offset_t > MAXIMUM_DELTA {
        renderer.offset_t += MAXIMUM_DELTA;
        for t_and_on in renderer.ts_and_ons.iter_mut() {
            if *t_and_on < (MAXIMUM_DELTA * 2) as f32 {
                if *t_and_on > (MAXIMUM_DELTA / 2) as f32 {
                    *t_and_on -= (MAXIMUM_DELTA / 2) as f32;
                } else if *t_and_on < (MAXIMUM_DELTA / 2) as f32 {
                    *t_and_on += (MAXIMUM_DELTA / 2) as f32;
                } else {
                    *t_and_on = (MAXIMUM_DELTA * 2) as f32;
                }
            }
        }
    }
}
