mod app;
mod css;
mod fuzzy;
mod song;

use seed::prelude::wasm_bindgen;
use seed::App;

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", app::init, app::update, app::view);
}
