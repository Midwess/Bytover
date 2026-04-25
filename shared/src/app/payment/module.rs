use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::operations::dialog::DialogOperation;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::capabilities::{UserCapabilities, EXPECTED_CAPABILITIES_VERSION};
use chrono::{DateTime, Duration, Utc};
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

pub type ProductId = String;

pub const CAPABILITIES_REFRESH_DEBOUNCE_SECS: i64 = 5;

#[derive(Debug, Clone, Default)]
pub struct PaymentModel {
    pub is_loading: bool,
    pub last_error: Option<String>,
    pub in_flight: Option<InFlight>,
    pub capabilities: Option<UserCapabilities>,
    pub is_refreshing_capabilities: bool,
    pub last_capabilities_refresh_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InFlight {
    Purchase(ProductId),
    Restore,
    ResumePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferSource {
    Cloud,
    P2P,
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
    RefreshCapabilities,
    #[serde(skip)]
    RefreshCapabilitiesStarted,
    #[serde(skip)]
    RefreshCapabilitiesFailed(String),
    #[serde(skip)]
    CapabilitiesLoaded(UserCapabilities),
    #[serde(skip)]
    ReportTransferBytesDelta { delta: u64, source: TransferSource },
    #[serde(skip)]
    ClearCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentViewModel {
    pub is_loading: bool,
    pub last_error_message: Option<String>,
    pub capabilities: Option<UserCapabilities>,
    pub cap_exceeded: bool,
}

fn derive_cap_exceeded(caps: Option<&UserCapabilities>) -> bool {
    let Some(caps) = caps else {
        return false;
    };
    if caps.is_paid() {
        return false;
    }
    let cap = caps.transfer_limits.total_transfer_bytes_lifetime_cap;
    if cap == 0 {
        return false;
    }
    caps.transfer_usage.total_transfer_bytes_used >= cap
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
                    .payment
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
            PaymentEvent::RefreshCapabilities => {
                if model.authentication.user.is_none() {
                    return Command::done();
                }
                if model.payment.is_refreshing_capabilities {
                    return Command::done();
                }
                if let Some(last) = model.payment.last_capabilities_refresh_at {
                    if Utc::now().signed_duration_since(last) < Duration::seconds(CAPABILITIES_REFRESH_DEBOUNCE_SECS) {
                        return Command::done();
                    }
                }

                model.payment.is_refreshing_capabilities = true;

                Command::new(|it| async move {
                    it.app().refresh_payment_capabilities().await;
                })
            }
            PaymentEvent::RefreshCapabilitiesStarted => {
                model.payment.is_refreshing_capabilities = true;
                Command::done()
            }
            PaymentEvent::RefreshCapabilitiesFailed(message) => {
                model.payment.is_refreshing_capabilities = false;
                log::warn!("Failed to refresh user capabilities: {message}");
                Command::done()
            }
            PaymentEvent::CapabilitiesLoaded(mut caps) => {
                if caps.capabilities_version > EXPECTED_CAPABILITIES_VERSION {
                    log::error!(
                        "Server sent capabilities_version={} but client expects <= {}; refusing to apply. Update the client.",
                        caps.capabilities_version,
                        EXPECTED_CAPABILITIES_VERSION
                    );
                    return Command::done();
                }
                if caps.capabilities_version < EXPECTED_CAPABILITIES_VERSION {
                    log::warn!(
                        "Server sent capabilities_version={} older than client expects ({}); applying with defaults where missing.",
                        caps.capabilities_version,
                        EXPECTED_CAPABILITIES_VERSION
                    );
                }

                let local_used = model
                    .payment
                    .capabilities
                    .as_ref()
                    .map(|c| c.transfer_usage.total_transfer_bytes_used)
                    .unwrap_or(0);
                caps.transfer_usage.total_transfer_bytes_used =
                    caps.transfer_usage.total_transfer_bytes_used.max(local_used);

                let limit = caps.shelf_limit();
                model.payment.capabilities.replace(caps);
                model.payment.is_refreshing_capabilities = false;
                model.payment.last_capabilities_refresh_at = Some(Utc::now());

                let cleanup = match limit {
                    Some(limit) => Command::handle_result(move |it| async move {
                        it.app().enforce_shelf_limit(limit as usize).await
                    }),
                    None => Command::done(),
                };

                Command::all(vec![Command::render(), cleanup])
            }
            PaymentEvent::ReportTransferBytesDelta { delta, source } => {
                if let Some(caps) = model.payment.capabilities.as_mut() {
                    caps.transfer_usage.total_transfer_bytes_used =
                        caps.transfer_usage.total_transfer_bytes_used.saturating_add(delta);
                }
                let render = Command::render();
                match source {
                    TransferSource::P2P => {
                        let rpc = Command::new(move |it| async move {
                            it.app().report_p2p_bytes_used(delta).await;
                        });
                        Command::all(vec![render, rpc])
                    }
                    TransferSource::Cloud => {
                        let refresh = Command::new(|it| async move {
                            it.app().notify_event(AppEvent::Payment(PaymentEvent::RefreshCapabilities));
                        });
                        Command::all(vec![render, refresh])
                    }
                }
            }
            PaymentEvent::ClearCapabilities => {
                model.payment.capabilities.take();
                model.payment.is_refreshing_capabilities = false;
                model.payment.last_capabilities_refresh_at = None;
                Command::render()
            }
        }
    }

    fn view(&self, model: &AppModel) -> PaymentViewModel {
        PaymentViewModel {
            is_loading: model.payment.is_loading,
            last_error_message: model.payment.last_error.clone(),
            capabilities: model.payment.capabilities.clone(),
            cap_exceeded: derive_cap_exceeded(model.payment.capabilities.as_ref()),
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
        model.payment.capabilities = Some(paid_capabilities());

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
        model.payment.capabilities = Some(UserCapabilities::free_defaults());

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
        model.payment.capabilities = Some(UserCapabilities::free_defaults());
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

    #[test]
    fn cap_exceeded_is_false_when_capabilities_none() {
        let app = BitBridge::default();
        let model = AppModel::default();

        let view = app.view(&model);
        let payment = view.payment.expect("payment view model");
        assert!(!payment.cap_exceeded);
        assert!(payment.capabilities.is_none());
    }

    #[test]
    fn cap_exceeded_is_false_for_paid_plan() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        let mut paid = paid_capabilities();
        paid.transfer_usage.total_transfer_bytes_used = 999_999_999_999;
        paid.transfer_limits.total_transfer_bytes_lifetime_cap = 100;
        model.payment.capabilities = Some(paid);

        let view = app.view(&model);
        let payment = view.payment.expect("payment view model");
        assert!(!payment.cap_exceeded);
    }

    #[test]
    fn cap_exceeded_is_false_for_free_below_cap() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        let mut free = UserCapabilities::free_defaults();
        free.transfer_usage.total_transfer_bytes_used = free.transfer_limits.total_transfer_bytes_lifetime_cap - 1;
        model.payment.capabilities = Some(free);

        let view = app.view(&model);
        let payment = view.payment.expect("payment view model");
        assert!(!payment.cap_exceeded);
    }

    #[test]
    fn cap_exceeded_is_true_for_free_at_or_above_cap() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        let mut free = UserCapabilities::free_defaults();
        free.transfer_usage.total_transfer_bytes_used = free.transfer_limits.total_transfer_bytes_lifetime_cap;
        model.payment.capabilities = Some(free.clone());

        let view = app.view(&model);
        let payment = view.payment.expect("payment view model");
        assert!(payment.cap_exceeded, "exact-cap should trigger gate");

        free.transfer_usage.total_transfer_bytes_used += 1;
        model.payment.capabilities = Some(free);
        let view = app.view(&model);
        let payment = view.payment.expect("payment view model");
        assert!(payment.cap_exceeded, "above-cap should trigger gate");
    }

    #[test]
    fn cap_exceeded_is_false_when_cap_is_zero_unlimited_sentinel() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        let mut free = UserCapabilities::free_defaults();
        free.transfer_limits.total_transfer_bytes_lifetime_cap = 0;
        free.transfer_usage.total_transfer_bytes_used = 100;
        model.payment.capabilities = Some(free);

        let view = app.view(&model);
        let payment = view.payment.expect("payment view model");
        assert!(!payment.cap_exceeded);
    }

    fn signed_in_model() -> AppModel {
        let mut model = AppModel::default();
        model.authentication.user = Some(crate::entities::user::User {
            id: 1,
            email: "test@example.com".to_owned(),
            name: "Test".to_owned(),
            avatar: String::new(),
        });
        model
    }

    #[test]
    fn refresh_capabilities_dispatches_get_capabilities_when_idle() {
        let app = BitBridge::default();
        let mut model = signed_in_model();

        let mut command = app.update(AppEvent::Payment(PaymentEvent::RefreshCapabilities), &mut model, &());
        let _ = collect_effects(&mut command);

        assert!(model.payment.is_refreshing_capabilities);
        assert!(model.payment.last_capabilities_refresh_at.is_none());
    }

    #[test]
    fn refresh_capabilities_skips_when_already_in_flight() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.payment.is_refreshing_capabilities = true;

        let mut command = app.update(AppEvent::Payment(PaymentEvent::RefreshCapabilities), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(model.payment.is_refreshing_capabilities);
        assert!(
            !effects.iter().any(|op| matches!(
                op,
                CoreOperation::Rpc(crate::app::operations::rpc::RpcOperation::GetCapabilities)
            )),
            "no GetCapabilities effect should be emitted while a refresh is in flight"
        );
    }

    #[test]
    fn refresh_capabilities_skips_when_within_debounce_window() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.payment.last_capabilities_refresh_at = Some(Utc::now());

        let mut command = app.update(AppEvent::Payment(PaymentEvent::RefreshCapabilities), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(!model.payment.is_refreshing_capabilities);
        assert!(
            !effects.iter().any(|op| matches!(
                op,
                CoreOperation::Rpc(crate::app::operations::rpc::RpcOperation::GetCapabilities)
            )),
            "no GetCapabilities effect should be emitted within the debounce window"
        );
    }

    #[test]
    fn refresh_capabilities_dispatches_after_debounce_elapses() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.payment.last_capabilities_refresh_at =
            Some(Utc::now() - Duration::seconds(CAPABILITIES_REFRESH_DEBOUNCE_SECS + 1));

        let mut command = app.update(AppEvent::Payment(PaymentEvent::RefreshCapabilities), &mut model, &());
        let _ = collect_effects(&mut command);

        assert!(model.payment.is_refreshing_capabilities);
    }

    #[test]
    fn refresh_capabilities_skips_when_signed_out() {
        let app = BitBridge::default();
        let mut model = AppModel::default();

        let mut command = app.update(AppEvent::Payment(PaymentEvent::RefreshCapabilities), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(!model.payment.is_refreshing_capabilities);
        assert!(effects.is_empty());
    }

    #[test]
    fn capabilities_loaded_clears_in_flight_and_stamps_refresh_time() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.payment.is_refreshing_capabilities = true;

        let _command = app.update(
            AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );

        assert!(!model.payment.is_refreshing_capabilities);
        assert!(model.payment.last_capabilities_refresh_at.is_some());
        assert_eq!(model.payment.capabilities.as_ref().map(|c| c.plan), Some(Plan::Paid));
    }

    #[test]
    fn capabilities_loaded_idempotent_under_repeated_dispatch() {
        let app = BitBridge::default();
        let mut model = signed_in_model();

        let _ = app.update(
            AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );
        let plan_after_first = model.payment.capabilities.as_ref().map(|c| c.plan);

        let _ = app.update(
            AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );
        let plan_after_second = model.payment.capabilities.as_ref().map(|c| c.plan);

        assert_eq!(plan_after_first, plan_after_second);
    }

    #[test]
    fn refresh_capabilities_failed_clears_in_flight_flag() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.payment.is_refreshing_capabilities = true;

        let _command = app.update(
            AppEvent::Payment(PaymentEvent::RefreshCapabilitiesFailed("boom".to_owned())),
            &mut model,
            &(),
        );

        assert!(!model.payment.is_refreshing_capabilities);
    }

    #[test]
    fn report_transfer_bytes_delta_cloud_advances_local_usage_and_queues_refresh() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        let mut free = UserCapabilities::free_defaults();
        free.transfer_usage.total_transfer_bytes_used = 1000;
        model.payment.capabilities = Some(free);

        let mut command = app.update(
            AppEvent::Payment(PaymentEvent::ReportTransferBytesDelta { delta: 500, source: TransferSource::Cloud }),
            &mut model,
            &(),
        );
        let _ = collect_effects(&mut command);

        assert_eq!(
            model.payment.capabilities.as_ref().unwrap().transfer_usage.total_transfer_bytes_used,
            1500
        );
    }

    #[test]
    fn report_transfer_bytes_delta_p2p_advances_local_usage() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        let mut free = UserCapabilities::free_defaults();
        free.transfer_usage.total_transfer_bytes_used = 1000;
        model.payment.capabilities = Some(free);

        let mut command = app.update(
            AppEvent::Payment(PaymentEvent::ReportTransferBytesDelta { delta: 500, source: TransferSource::P2P }),
            &mut model,
            &(),
        );
        let _ = collect_effects(&mut command);

        assert_eq!(
            model.payment.capabilities.as_ref().unwrap().transfer_usage.total_transfer_bytes_used,
            1500
        );
    }

    #[test]
    fn capabilities_loaded_one_way_ratchet_holds_local_higher_value() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        let mut local = UserCapabilities::free_defaults();
        local.transfer_usage.total_transfer_bytes_used = 21 * 1024 * 1024 * 1024;
        model.payment.capabilities = Some(local);

        let mut stale_server = UserCapabilities::free_defaults();
        stale_server.transfer_usage.total_transfer_bytes_used = 19 * 1024 * 1024 * 1024;

        let _command = app.update(
            AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(stale_server)),
            &mut model,
            &(),
        );

        assert_eq!(
            model.payment.capabilities.as_ref().unwrap().transfer_usage.total_transfer_bytes_used,
            21 * 1024 * 1024 * 1024,
            "ratchet should hold the local higher value when server returns stale lower count"
        );
    }

    #[test]
    fn clear_capabilities_resets_payment_capability_state() {
        let app = BitBridge::default();
        let mut model = AppModel::default();
        model.payment.capabilities = Some(paid_capabilities());
        model.payment.is_refreshing_capabilities = true;
        model.payment.last_capabilities_refresh_at = Some(Utc::now());

        let _command = app.update(AppEvent::Payment(PaymentEvent::ClearCapabilities), &mut model, &());

        assert!(model.payment.capabilities.is_none());
        assert!(!model.payment.is_refreshing_capabilities);
        assert!(model.payment.last_capabilities_refresh_at.is_none());
    }
}


