use bevy_egui::egui;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarketUiCommand {
    pub side: TradeSide,
    pub resource: String,
    pub quantity: u32,
    pub price: f64,
}

#[derive(Debug, Clone)]
pub struct MarketPanelState {
    pub resource: String,
    pub quantity: u32,
    pub price: f64,
    pub side: TradeSide,
    pub auto_submit: bool,
    pub opened_logged: bool,
}

impl Default for MarketPanelState {
    fn default() -> Self {
        Self {
            resource: "Steel".to_string(),
            quantity: 10,
            price: 100.0,
            side: TradeSide::Buy,
            auto_submit: false,
            opened_logged: false,
        }
    }
}

impl MarketPanelState {
    fn build_command(&self) -> MarketUiCommand {
        MarketUiCommand {
            side: self.side,
            resource: self.resource.clone(),
            quantity: self.quantity,
            price: self.price,
        }
    }
}

/// Renders a simple market panel and returns a command when submitted.
pub fn render_market_panel(
    ui: &mut egui::Ui,
    state: &mut MarketPanelState,
) -> Option<MarketUiCommand> {
    if !state.opened_logged {
        info!("Market UI opened");
        state.opened_logged = true;
    }

    let mut emitted = None;
    ui.heading("Market");
    ui.label("Submit buy/sell orders against the market price.");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Resource:");
        ui.text_edit_singleline(&mut state.resource);
    });
    ui.horizontal(|ui| {
        ui.label("Quantity:");
        ui.add(egui::DragValue::new(&mut state.quantity).clamp_range(1..=100_000));
    });
    ui.horizontal(|ui| {
        ui.label("Price:");
        ui.add(
            egui::DragValue::new(&mut state.price)
                .clamp_range(0.0..=1_000_000.0)
                .speed(1.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label("Side:");
        ui.selectable_value(&mut state.side, TradeSide::Buy, "Buy");
        ui.selectable_value(&mut state.side, TradeSide::Sell, "Sell");
    });

    let label = match state.side {
        TradeSide::Buy => "Submit Buy",
        TradeSide::Sell => "Submit Sell",
    };

    if ui.button(label).clicked() || std::mem::take(&mut state.auto_submit) {
        let cmd = state.build_command();
        info!(
            "Trade order submitted: {:?} {} {} @ {}",
            cmd.side, cmd.quantity, cmd.resource, cmd.price
        );
        emitted = Some(cmd);
    }

    emitted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_matches_state() {
        let state = MarketPanelState {
            resource: "Lithium".into(),
            quantity: 25,
            price: 123.4,
            side: TradeSide::Sell,
            auto_submit: false,
            opened_logged: false,
        };
        let cmd = state.build_command();
        assert_eq!(cmd.resource, "Lithium");
        assert_eq!(cmd.quantity, 25);
        assert!((cmd.price - 123.4).abs() < 1e-6);
        assert_eq!(cmd.side, TradeSide::Sell);
    }

    #[test]
    fn render_returns_command_when_auto_submit() {
        let ctx = egui::Context::default();
        let mut state = MarketPanelState {
            resource: "Copper".into(),
            quantity: 5,
            price: 10.0,
            side: TradeSide::Buy,
            auto_submit: true,
            opened_logged: false,
        };

        ctx.begin_frame(egui::RawInput::default());
        let cmd = egui::CentralPanel::default()
            .show(&ctx, |ui| render_market_panel(ui, &mut state))
            .inner;
        let _ = ctx.end_frame();

        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.side, TradeSide::Buy);
        assert_eq!(cmd.resource, "Copper");
        assert_eq!(cmd.quantity, 5);
        assert_eq!(cmd.price, 10.0);
    }
}
