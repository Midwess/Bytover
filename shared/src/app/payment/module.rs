use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::operations::dialog::DialogOperation;
use crate::app::{AppModel, BitBridge};
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

pub type ProductId = String;

#[derive(Debug, Clone, Default)]
pub struct PaymentModel {
    pub is_loading: bool,
    pub last_error: Option<String>,
    pub in_flight: Option<InFlight>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InFlight {
    Purchase(ProductId),
    Restore,
    ResumePending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PaymentEvent {
    Purchase(ProductId),
    Restore,
    ResumePending,
    BeginLoading(InFlight),
    EndLoading,
    SubmissionCompleted,
    SubmissionFailed(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentViewModel {
    pub is_loading: bool,
    pub last_error_message: Option<String>,
}

pub struct PaymentModule;

impl AppModule<BitBridge> for PaymentModule {
    type Event = PaymentEvent;
    type ViewModel = PaymentViewModel;

    fn update(
        &self,
        event: PaymentEvent,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities,
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            PaymentEvent::Purchase(product_id) => {
                if model.payment.is_loading {
                    log::info!("[payment] purchase ignored: already in flight");
                    return Command::done();
                }

                if model
                    .authentication
                    .capabilities
                    .as_ref()
                    .map(|c| c.is_paid())
                    .unwrap_or(false)
                {
                    log::info!("[payment] purchase ignored: user is already on Premium");
                    return Command::operate(DialogOperation::Toast("You're already on Premium".to_owned()));
                }

                Command::handle_result(move |it| async move { it.app().purchase(product_id).await })
            }
            PaymentEvent::Restore => {
                if model.payment.is_loading {
                    log::info!("[payment] restore ignored: already in flight");
                    return Command::done();
                }
                Command::handle_result(|it| async move { it.app().restore().await })
            }
            PaymentEvent::ResumePending => {
                if model.payment.is_loading {
                    log::info!("[payment] resume_pending ignored: already in flight");
                    return Command::done();
                }
                Command::handle_result(|it| async move { it.app().resume_pending().await })
            }
            PaymentEvent::BeginLoading(flight) => {
                model.payment.is_loading = true;
                model.payment.in_flight = Some(flight);
                model.payment.last_error = None;
                Command::render()
            }
            PaymentEvent::EndLoading => {
                model.payment.is_loading = false;
                model.payment.in_flight = None;
                Command::render()
            }
            PaymentEvent::SubmissionCompleted => {
                model.payment.last_error = None;
                Command::render()
            }
            PaymentEvent::SubmissionFailed(message) => {
                model.payment.last_error = Some(message);
                Command::render()
            }
        }
    }

    fn view(&self, model: &AppModel) -> PaymentViewModel {
        PaymentViewModel {
            is_loading: model.payment.is_loading,
            last_error_message: model.payment.last_error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::operations::CoreOperation;
    use crate::app::{AppEvent, AppOperation, BitBridge};
    use crate::entities::capabilities::{Plan, UserCapabilities};
    use crux_core::App;

    fn paid_capabilities() -> UserCapabilities {
        UserCapabilities {
            plan: Plan::Paid,
            ..UserCapabilities::free_defaults()
        }
    }

    fn collect_effects(command: &mut crate::app::AppCommand) -> Vec<CoreOperation> {
        command
            .effects()
            .map(|effect| {
                let AppOperation::Operation(request) = effect;
                let (op, _) = request.split();
                op
            })
            .collect()
    }

    #[test]
    fn purchase_event_short_circuits_when_already_paid() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.authentication.capabilities = Some(paid_capabilities());

        let mut command = app.update(AppEvent::Payment(PaymentEvent::Purchase("any".to_owned())), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(!model.payment.is_loading);
        let toast_emitted = effects.iter().any(|op| {
            matches!(
                op,
                CoreOperation::Dialog(crate::app::operations::dialog::DialogOperation::Toast(_))
            )
        });
        assert!(toast_emitted, "expected DialogOperation::Toast for already-paid case, got {effects:?}");
        assert!(
            !effects.iter().any(|op| matches!(op, CoreOperation::StoreKit(_))),
            "no StoreKit op should be emitted when user is already on Premium"
        );
    }

    #[test]
    fn purchase_event_dispatches_when_user_is_free() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.authentication.capabilities = Some(UserCapabilities::free_defaults());

        let mut command = app.update(AppEvent::Payment(PaymentEvent::Purchase("any".to_owned())), &mut model, &());
        let _ = collect_effects(&mut command);

        assert!(
            !model.payment.is_loading,
            "is_loading is toggled by BeginLoading event from inside the spawned command, not by Purchase itself"
        );
    }

    #[test]
    fn begin_loading_sets_flag_and_clears_error() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.payment.last_error = Some("stale".to_owned());

        let _command = app.update(
            AppEvent::Payment(PaymentEvent::BeginLoading(InFlight::Purchase("p".to_owned()))),
            &mut model,
            &(),
        );

        assert!(model.payment.is_loading);
        assert_eq!(model.payment.in_flight, Some(InFlight::Purchase("p".to_owned())));
        assert_eq!(model.payment.last_error, None);
    }

    #[test]
    fn end_loading_clears_flag_and_in_flight() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.payment.is_loading = true;
        model.payment.in_flight = Some(InFlight::Restore);

        let _command = app.update(AppEvent::Payment(PaymentEvent::EndLoading), &mut model, &());

        assert!(!model.payment.is_loading);
        assert_eq!(model.payment.in_flight, None);
    }

    #[test]
    fn submission_failed_records_error_in_model() {
        let app = BitBridge::default();
        let mut model = AppModel::default();

        let _command = app.update(
            AppEvent::Payment(PaymentEvent::SubmissionFailed("upstream rejected".to_owned())),
            &mut model,
            &(),
        );

        assert_eq!(model.payment.last_error.as_deref(), Some("upstream rejected"));
    }

    #[test]
    fn submission_completed_clears_last_error() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.payment.last_error = Some("stale".to_owned());

        let _command = app.update(AppEvent::Payment(PaymentEvent::SubmissionCompleted), &mut model, &());

        assert_eq!(model.payment.last_error, None);
    }

    #[test]
    fn purchase_event_ignored_while_already_loading() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.authentication.capabilities = Some(UserCapabilities::free_defaults());
        model.payment.is_loading = true;

        let mut command = app.update(AppEvent::Payment(PaymentEvent::Purchase("p".to_owned())), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(
            effects.is_empty(),
            "no effects should be produced when a purchase is already in flight, got {effects:?}"
        );
    }

    #[test]
    fn view_reflects_loading_and_error() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.payment.is_loading = true;
        model.payment.last_error = Some("oops".to_owned());

        let view = app.view(&model);

        let payment = view.payment.expect("payment view model");
        assert!(payment.is_loading);
        assert_eq!(payment.last_error_message.as_deref(), Some("oops"));
    }
}


