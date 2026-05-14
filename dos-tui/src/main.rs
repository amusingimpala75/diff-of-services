mod app;

fn main() -> anyhow::Result<()> {
    ratatui::run(|terminal| app::App::default().run(terminal))
}
