use bevy_egui::egui;
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum BankingUiCommand {
    TakeLoan {
        principal: f64,
        interest_rate_pct: f64,
        term_turns: u32,
    },
    RepayLoan {
        amount: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankingMode {
    TakeLoan,
    RepayLoan,
}

#[derive(Debug, Clone)]
pub struct BankingPanelState {
    pub mode: BankingMode,
    pub principal: f64,
    pub interest_rate_pct: f64,
    pub term_turns: u32,
    pub payment_amount: f64,
    pub auto_submit: bool,
    pub opened_logged: bool,
}

impl Default for BankingPanelState {
    fn default() -> Self {
        Self {
            mode: BankingMode::TakeLoan,
            principal: 1000.0,
            interest_rate_pct: 5.0,
            term_turns: 12,
            payment_amount: 250.0,
            auto_submit: false,
            opened_logged: false,
        }
    }
}

impl BankingPanelState {
    fn build_command(&self) -> BankingUiCommand {
        match self.mode {
            BankingMode::TakeLoan => BankingUiCommand::TakeLoan {
                principal: self.principal,
                interest_rate_pct: self.interest_rate_pct,
                term_turns: self.term_turns,
            },
            BankingMode::RepayLoan => BankingUiCommand::RepayLoan {
                amount: self.payment_amount,
            },
        }
    }
}

/// Renders a simple banking panel and returns a command when submitted.
pub fn render_banking_panel(
    ui: &mut egui::Ui,
    state: &mut BankingPanelState,
) -> Option<BankingUiCommand> {
    if !state.opened_logged {
        info!("Banking UI opened");
        state.opened_logged = true;
    }

    let mut emitted = None;
    ui.heading("Banking");
    ui.label("Take loans or make repayments.");
    ui.separator();

    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.mode, BankingMode::TakeLoan, "Take Loan");
        ui.selectable_value(&mut state.mode, BankingMode::RepayLoan, "Repay Loan");
    });

    match state.mode {
        BankingMode::TakeLoan => {
            ui.horizontal(|ui| {
                ui.label("Principal:");
                ui.add(
                    egui::DragValue::new(&mut state.principal)
                        .clamp_range(0.0..=5_000_000.0)
                        .speed(100.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Interest %:");
                ui.add(
                    egui::DragValue::new(&mut state.interest_rate_pct)
                        .clamp_range(0.0..=100.0)
                        .speed(0.1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Term (turns):");
                ui.add(egui::DragValue::new(&mut state.term_turns).clamp_range(1..=600));
            });
        }
        BankingMode::RepayLoan => {
            ui.horizontal(|ui| {
                ui.label("Payment:");
                ui.add(
                    egui::DragValue::new(&mut state.payment_amount)
                        .clamp_range(0.0..=5_000_000.0)
                        .speed(100.0),
                );
            });
        }
    }

    if ui.button("Submit").clicked() || std::mem::take(&mut state.auto_submit) {
        let cmd = state.build_command();
        match &cmd {
            BankingUiCommand::TakeLoan {
                principal,
                interest_rate_pct,
                term_turns,
            } => {
                info!(
                    "Banking order submitted: loan {} @ {}% over {} turns",
                    principal, interest_rate_pct, term_turns
                );
            }
            BankingUiCommand::RepayLoan { amount } => {
                info!("Banking order submitted: repay {}", amount);
            }
        }
        emitted = Some(cmd);
    }

    emitted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_take_loan_command() {
        let state = BankingPanelState {
            mode: BankingMode::TakeLoan,
            principal: 2500.0,
            interest_rate_pct: 3.5,
            term_turns: 24,
            payment_amount: 0.0,
            auto_submit: false,
            opened_logged: false,
        };
        let cmd = state.build_command();
        assert_eq!(
            cmd,
            BankingUiCommand::TakeLoan {
                principal: 2500.0,
                interest_rate_pct: 3.5,
                term_turns: 24
            }
        );
    }

    #[test]
    fn render_emits_repay_command_on_auto_submit() {
        let ctx = egui::Context::default();
        let mut state = BankingPanelState {
            mode: BankingMode::RepayLoan,
            principal: 0.0,
            interest_rate_pct: 0.0,
            term_turns: 0,
            payment_amount: 150.0,
            auto_submit: true,
            opened_logged: false,
        };

        ctx.begin_frame(egui::RawInput::default());
        let cmd = egui::CentralPanel::default()
            .show(&ctx, |ui| render_banking_panel(ui, &mut state))
            .inner;
        let _ = ctx.end_frame();

        assert_eq!(cmd, Some(BankingUiCommand::RepayLoan { amount: 150.0 }));
    }
}
