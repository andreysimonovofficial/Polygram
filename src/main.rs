mod app;
mod extensions;
mod local_store;
mod model;
mod network;

use app::PolygramApp;

fn main() -> eframe::Result<()> {
    let network = network::start();
    let mut options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Polygram")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    options.wgpu_options.desired_maximum_frame_latency = Some(1);

    eframe::run_native(
        "Polygram",
        options,
        Box::new(move |creation_context| {
            let renderer = creation_context
                .wgpu_render_state
                .as_ref()
                .map(|state| {
                    let info = state.adapter.get_info();
                    format!(
                        "{} · {:?} · {:?}",
                        info.name, info.backend, info.device_type
                    )
                })
                .unwrap_or_else(|| "GPU adapter unavailable".to_owned());
            Ok(Box::new(PolygramApp::new(network, renderer)?))
        }),
    )
}
