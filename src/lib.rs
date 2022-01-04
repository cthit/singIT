mod app;
mod css;
mod fuzzy;
mod query;
mod song;

use seed::prelude::wasm_bindgen;
use seed::App;

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", app::init, app::update, app::view);
}
