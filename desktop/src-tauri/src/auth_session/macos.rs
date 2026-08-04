use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObject as RuntimeNSObject, ProtocolObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_authentication_services::{
    ASPresentationAnchor,
    ASWebAuthenticationPresentationContextProviding,
    ASWebAuthenticationSession,
    ASWebAuthenticationSessionErrorCode
};
use objc2_foundation::{NSError, NSObject, NSObjectProtocol, NSString, NSURL};
use shared::app::operations::webview::AuthenticationSessionOutcome;
use std::cell::RefCell;
use std::rc::Rc;
use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;

struct PresentationContextIvars {
    anchor: Retained<RuntimeNSObject>
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "BytoverAuthenticationPresentationContext"]
    #[ivars = PresentationContextIvars]
    struct PresentationContext;

    unsafe impl NSObjectProtocol for PresentationContext {}

    unsafe impl ASWebAuthenticationPresentationContextProviding for PresentationContext {
        #[unsafe(method_id(presentationAnchorForWebAuthenticationSession:))]
        fn presentation_anchor(&self, _session: &ASWebAuthenticationSession) -> Retained<ASPresentationAnchor> {
            self.ivars().anchor.clone()
        }
    }
);

impl PresentationContext {
    fn new(mtm: MainThreadMarker, anchor: Retained<RuntimeNSObject>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(PresentationContextIvars { anchor });
        unsafe { msg_send![super(this), init] }
    }
}

struct ActiveSession {
    _session: Retained<ASWebAuthenticationSession>,
    _presentation_context: Retained<PresentationContext>
}

thread_local! {
    static ACTIVE_SESSION: RefCell<Option<ActiveSession>> = const { RefCell::new(None) };
}

pub(crate) async fn authenticate(app_handle: AppHandle, url: String, callback_scheme: String) -> AuthenticationSessionOutcome {
    let (sender, receiver) = oneshot::channel();
    let scheduled = app_handle.clone().run_on_main_thread(move || {
        start_on_main_thread(app_handle, url, callback_scheme, sender);
    });

    if scheduled.is_err() {
        return AuthenticationSessionOutcome::Failed {
            code: "main_thread_unavailable".to_string()
        };
    }

    receiver.await.unwrap_or(AuthenticationSessionOutcome::Failed {
        code: "session_interrupted".to_string()
    })
}

#[allow(deprecated)]
fn start_on_main_thread(
    app_handle: AppHandle,
    url: String,
    callback_scheme: String,
    sender: oneshot::Sender<AuthenticationSessionOutcome>
) {
    let already_active = ACTIVE_SESSION.with(|active| active.borrow().is_some());
    if already_active {
        let _ = sender.send(AuthenticationSessionOutcome::Failed {
            code: "session_already_active".to_string()
        });
        return;
    }

    let Some(mtm) = MainThreadMarker::new() else {
        let _ = sender.send(AuthenticationSessionOutcome::Failed {
            code: "main_thread_unavailable".to_string()
        });
        return;
    };
    let Some(window) = app_handle.get_webview_window("auth") else {
        let _ = sender.send(AuthenticationSessionOutcome::Failed {
            code: "presentation_window_unavailable".to_string()
        });
        return;
    };
    let Ok(window_ptr) = window.ns_window() else {
        let _ = sender.send(AuthenticationSessionOutcome::Failed {
            code: "presentation_window_unavailable".to_string()
        });
        return;
    };
    let Some(anchor) = (unsafe { Retained::<RuntimeNSObject>::retain(window_ptr.cast()) }) else {
        let _ = sender.send(AuthenticationSessionOutcome::Failed {
            code: "presentation_window_unavailable".to_string()
        });
        return;
    };
    let Some(authentication_url) = NSURL::URLWithString(&NSString::from_str(&url)) else {
        let _ = sender.send(AuthenticationSessionOutcome::Failed {
            code: "invalid_authentication_url".to_string()
        });
        return;
    };

    let sender = Rc::new(RefCell::new(Some(sender)));
    let completion_sender = sender.clone();
    let completion = RcBlock::new(move |callback_url: *mut NSURL, error: *mut NSError| {
        let outcome = completion_outcome(callback_url, error);
        ACTIVE_SESSION.with(|active| {
            active.borrow_mut().take();
        });
        if let Some(sender) = completion_sender.borrow_mut().take() {
            let _ = sender.send(outcome);
        }
    });
    let session = unsafe {
        ASWebAuthenticationSession::initWithURL_callbackURLScheme_completionHandler(
            ASWebAuthenticationSession::alloc(),
            &authentication_url,
            Some(&NSString::from_str(&callback_scheme)),
            RcBlock::as_ptr(&completion)
        )
    };
    let presentation_context = PresentationContext::new(mtm, anchor);
    let presentation_protocol = ProtocolObject::from_ref(&*presentation_context);
    unsafe {
        session.setPresentationContextProvider(Some(presentation_protocol));
    }

    ACTIVE_SESSION.with(|active| {
        active.borrow_mut().replace(ActiveSession {
            _session: session.clone(),
            _presentation_context: presentation_context.clone()
        });
    });

    let started = unsafe { session.start() };
    if !started {
        ACTIVE_SESSION.with(|active| {
            active.borrow_mut().take();
        });
        if let Some(sender) = sender.borrow_mut().take() {
            let _ = sender.send(AuthenticationSessionOutcome::Failed {
                code: "session_start_failed".to_string()
            });
        }
        return;
    }

}

fn completion_outcome(callback_url: *mut NSURL, error: *mut NSError) -> AuthenticationSessionOutcome {
    if !callback_url.is_null() {
        let callback_url = unsafe { &*callback_url };
        if let Some(callback_url) = callback_url.absoluteString() {
            return AuthenticationSessionOutcome::Completed {
                callback_url: callback_url.to_string()
            };
        }
        return AuthenticationSessionOutcome::Failed {
            code: "invalid_callback_url".to_string()
        };
    }

    if !error.is_null() && unsafe { (&*error).code() } == ASWebAuthenticationSessionErrorCode::CanceledLogin.0 {
        AuthenticationSessionOutcome::Cancelled
    } else {
        AuthenticationSessionOutcome::Failed {
            code: "web_authentication_failed".to_string()
        }
    }
}
