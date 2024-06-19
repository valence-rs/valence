use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use egui_dock::{DockArea, NodeIndex, Style, Tree};
use packet_inspector::Proxy;
use tokio::task::JoinHandle;

use crate::shared_state::{Event, SharedState};

mod connection;
mod filter;
mod hex_viewer;
mod packet_list;
mod text_viewer;

pub(crate) trait View {
    fn ui(&mut self, ui: &mut egui::Ui, shared_state: &mut SharedState);
}

/// Something to view
pub(crate) trait Tab: View {
    fn new() -> Self
    where
        Self: Sized;

    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;
}

struct TabViewer {
    shared_state: Arc<RwLock<SharedState>>,
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Box<dyn Tab>;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui, &mut self.shared_state.write().unwrap());
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name().into()
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

pub(crate) struct GuiApp {
    tree: Tree<Box<dyn Tab>>,
    shared_state: Arc<RwLock<SharedState>>,
    tab_viewer: TabViewer,
}

impl GuiApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();

        // Default Application Layout
        let mut tree: Tree<Box<dyn Tab>> = Tree::new(vec![Box::new(connection::Connection::new())]);

        let [a, b] = tree.split_right(
            NodeIndex::root(),
            0.3,
            vec![Box::new(packet_list::PacketList::new())],
        );

        let [_, _] = tree.split_below(a, 0.25, vec![Box::new(filter::Filter::new())]);
        let [_, _] = tree.split_below(
            b,
            0.5,
            vec![
                Box::new(text_viewer::TextView::new()),
                Box::new(hex_viewer::HexView::new()),
            ],
        );

        // Persistent Storage
        let mut shared_state = SharedState::new(ctx);

        if let Some(storage) = cc.storage {
            if let Some(value) = eframe::get_value::<SharedState>(storage, eframe::APP_KEY) {
                shared_state = value.merge(shared_state);
            }
        }

        let autostart = shared_state.autostart;
        let shared_state = Arc::new(RwLock::new(shared_state));

        // Event Handling
        handle_events(shared_state.clone());

        if autostart {
            let state = shared_state.read().unwrap();
            state.send_event(Event::StartListening);
        }

        // Consumer thread

        // Tab Viewer
        let tab_viewer = TabViewer {
            shared_state: shared_state.clone(),
        };

        Self { tree, shared_state, tab_viewer }
    }
}

impl eframe::App for GuiApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            eframe::APP_KEY,
            &*self.shared_state.read().unwrap(),
        );
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        DockArea::new(&mut self.tree)
            .show_add_buttons(false)
            .show_add_popup(false)
            .show_close_buttons(false)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut self.tab_viewer);
    }
}

// This function is getting waaaay too complicated and messy
fn handle_events(state: Arc<RwLock<SharedState>>) {
    tokio::spawn(async move {
        let mut proxy_thread: Option<JoinHandle<_>> = None;

        let receiver = state.write().unwrap().receiver.take().unwrap();
        while let Ok(event) = receiver.recv_async().await {
            match event {
                Event::StartListening => {
                    let mut w_state = state.write().unwrap();
                    if w_state.is_listening {
                        continue;
                    }

                    let Ok(listener_addr) = w_state.listener_addr.parse::<SocketAddr>() else {
                        w_state.is_listening = false;
                        continue;
                    };
                    let Ok(server_addr) = w_state.server_addr.parse::<SocketAddr>() else {
                        w_state.is_listening = false;
                        continue;
                    };

                    let state = state.clone();

                    proxy_thread = Some(tokio::spawn(async move {
                        let proxy = Proxy::start(listener_addr, server_addr).await?;
                        let receiver = proxy.subscribe().await;

                        while let Ok(packet) = receiver.recv_async().await {
                            let state = state.read().unwrap();
                            state.packets.write().unwrap().push(packet);
                            state.send_event(Event::PacketReceived);
                        }

                        Ok::<(), anyhow::Error>(())
                    }));

                    w_state.is_listening = true;
                }
                Event::StopListening => {
                    let mut state = state.write().unwrap();
                    if !state.is_listening {
                        continue;
                    }

                    if let Some(proxy_thread) = proxy_thread.take() {
                        proxy_thread.abort();
                    }

                    state.is_listening = false;
                }
                Event::PacketReceived => {
                    // Refresh UI
                    if let Some(ctx) = &state.read().unwrap().ctx {
                        ctx.request_repaint();
                    }
                }
            }
        }
    });
}
