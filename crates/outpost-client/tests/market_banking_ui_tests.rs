use outpost_client::ui::banking::*;
use outpost_client::ui::market::*;
use bevy_egui::egui;

#[test]
fn market_panel_emits_buy_command() {
    let ctx = egui::Context::default();
    let mut state = MarketPanelState {
        resource: "Steel".into(),
        quantity: 20,
        price: 45.0,
        side: TradeSide::Buy,
        auto_submit: true,
        opened_logged: false,
    };

    ctx.begin_frame(egui::RawInput::default());
    let cmd = egui::CentralPanel::default()
        .show(&ctx, |ui| render_market_panel(ui, &mut state))
        .inner
        .expect("Expected command");
    let _ = ctx.end_frame();

    assert_eq!(cmd.side, TradeSide::Buy);
    assert_eq!(cmd.resource, "Steel");
    assert_eq!(cmd.quantity, 20);
    assert_eq!(cmd.price, 45.0);
}

#[test]
fn banking_panel_emits_take_loan_command() {
    let ctx = egui::Context::default();
    let mut state = BankingPanelState {
        mode: BankingMode::TakeLoan,
        principal: 1500.0,
        interest_rate_pct: 4.0,
        term_turns: 18,
        payment_amount: 0.0,
        auto_submit: true,
        opened_logged: false,
    };

    ctx.begin_frame(egui::RawInput::default());
    let cmd = egui::CentralPanel::default()
        .show(&ctx, |ui| render_banking_panel(ui, &mut state))
        .inner
        .expect("Expected command");
    let _ = ctx.end_frame();

    assert_eq!(cmd, BankingUiCommand::TakeLoan { principal: 1500.0, interest_rate_pct: 4.0, term_turns: 18 });
}
