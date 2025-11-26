use ggez::{
    Context,
    graphics::{Image, ImageFormat},
};

use crate::collections::tile_grid::{Empty, TileGrid};

pub struct Pixels {
    colors: TileGrid<PixelColor>,
    data: TileGrid<PixelData>,
    image: Option<Image>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct PixelColor(pub [u8; 4]);

impl Empty for PixelColor {
    fn empty() -> &'static Self {
        &PixelColor([0; 4])
    }

    fn is_empty(&self) -> bool {
        self.0 == [0; 4]
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct PixelData {}

impl Empty for PixelData {
    fn empty() -> &'static Self {
        &PixelData {}
    }

    fn is_empty(&self) -> bool {
        true
    }
}

impl Default for Pixels {
    fn default() -> Self {
        Self {
            colors: TileGrid::default(),
            data: TileGrid::default(),
            image: None,
        }
    }
}

impl Pixels {
    pub fn update_image(&mut self, ctx: &mut Context) {
        let new_size = self.colors.bounds().size.map(|x| x as u32);

        // HACK: For some reason ggez doesn't let us change the contents of an image...
        if new_size.x == 0 || new_size.y == 0 {
            self.image = None;
        } else {
            self.image = Some(Image::from_pixels(
                ctx,
                self.colors_as_slice().as_flattened(),
                ImageFormat::Rgba8UnormSrgb,
                new_size.x,
                new_size.y,
            ));
        }
    }

    pub fn colors_as_slice(&self) -> &[[u8; 4]] {
        unsafe { std::mem::transmute::<&[PixelColor], &[[u8; 4]]>(self.colors.as_slice()) }
    }

    pub fn colors_as_slice_mut(&mut self) -> &mut [[u8; 4]] {
        unsafe {
            std::mem::transmute::<&mut [PixelColor], &mut [[u8; 4]]>(self.colors.as_slice_mut())
        }
    }
}
