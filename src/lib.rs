use zed_extension_api::{self as zed};

struct TestaustimeExtension;

impl TestaustimeExtension {}

impl zed::Extension for TestaustimeExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(TestaustimeExtension);
