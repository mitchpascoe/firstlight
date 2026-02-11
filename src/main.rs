mod app;
mod esa;
mod preview;
mod wallpaper;

use anyhow::Result;

use app::App;

fn main() -> Result<()> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        hook(info);
    }));

    let mut terminal = ratatui::init();
    let result = App::new()?.run(&mut terminal);
    ratatui::restore();
    result
}
