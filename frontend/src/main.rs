mod app;
mod category;
mod css;
mod custom_list;
mod fetch;
mod fuzzy;
mod query;
mod song;

use seed::App;

fn main() {
    App::start("app", app::init, app::update, app::view);
}
