use crate::user_storage::{UserAccount, UserId};
use futures_util::TryFutureExt;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use time::OffsetDateTime;
use tokio::{stream::StreamExt, sync::RwLock};
use uuid::Uuid;
use warp::http::HeaderValue;

#[derive(Clone)]
pub struct Sessions(Arc<RwLock<SessionInner>>);

impl Sessions {
    pub fn new(gc_time: Duration) -> Self {
        let ret = Self(Default::default());
        let weakling = Arc::downgrade(&ret.0);
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(gc_time);
            while interval.next().await.is_some() {
                if let Some(this) = weakling.upgrade() {
                    let mut storage = this.write().await;
                    storage.gc();
                } else {
                    return;
                }
            }
        });

        ret
    }

    pub async fn login(&self, account: UserAccount) -> LoginSession {
        let uuid = {
            let storage = self.0.read().await;
            loop {
                let uuid = Uuid::new_v4();
                if !storage.sessions.contains_key(&uuid) {
                    break uuid;
                }
            }
        };
        let mut storage = self.0.write().await;
        let stale_session = storage
            .users_logged_in
            .get_mut(&account.id)
            .map(|stale_session| std::mem::replace(stale_session, uuid));

        if let Some(stale_session) = stale_session {
            storage.sessions.remove(&stale_session);
        } else {
            storage.users_logged_in.insert(account.id, uuid);
        }

        let now = time::OffsetDateTime::now_utc();

        storage.sessions.insert(
            uuid,
            SessionData {
                expiry: now + time::Duration::seconds(3600),
                account,
            },
        );

        LoginSession {
            uuid,
            expiry_time: time::Duration::seconds(3600),
        }
    }

    pub async fn get_user_data(&self, session_id: Uuid) -> Option<UserAccount> {
        enum UserSession {
            Expired,
            Live(UserAccount),
        }

        let user_data = {
            let storage = self.0.read().await;
            let now = OffsetDateTime::now_utc();

            match storage.sessions.get(&session_id) {
                Some(data) if data.expired(now) => Some(UserSession::Expired),
                Some(SessionData { account, .. }) => Some(UserSession::Live(account.clone())),
                _ => None,
            }
        };

        match user_data {
            Some(UserSession::Live(account)) => Some(account),
            Some(UserSession::Expired) => {
                self.remove_session(session_id).await;
                None
            }
            _ => None,
        }
    }

    async fn remove_session(&self, session_id: Uuid) {
        self.0.write().await.remove_session(session_id);
    }

    pub async fn logout(&self, user_id: UserId) {
        let mut storage = self.0.write().await;
        storage.remove_user(user_id);
    }
}

#[derive(Default)]
pub struct SessionInner {
    sessions: BTreeMap<Uuid, SessionData>,
    users_logged_in: BTreeMap<UserId, Uuid>,
}

impl SessionInner {
    fn remove_session(&mut self, session_id: Uuid) {
        if let Some(entry) = self.sessions.remove(&session_id) {
            self.users_logged_in.remove(&entry.account.id);
        }
    }

    fn remove_user(&mut self, user_id: UserId) {
        if let Some(session_id) = self.users_logged_in.remove(&user_id) {
            self.sessions.remove(&session_id);
        }
    }

    fn gc(&mut self) {
        let now = OffsetDateTime::now_utc();
        // maybe use a probabilistic algorithm so I don't have to iter over everything?
        let to_remove = self
            .sessions
            .iter()
            .filter_map(
                |(id, data)| {
                    if data.expiry < now {
                        Some(*id)
                    } else {
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        for id in to_remove {
            self.remove_session(id);
        }
    }
}

pub struct SessionData {
    account: crate::user_storage::UserAccount,
    expiry: OffsetDateTime,
}

impl SessionData {
    fn expired(&self, now: OffsetDateTime) -> bool {
        self.expiry < now
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Received invalid cookie")]
    CorruptedCookie,

    #[error("Session required")]
    SessionRequired { access_url: String },
}

impl From<uuid::Error> for Error {
    fn from(_: uuid::Error) -> Self {
        Error::CorruptedCookie
    }
}

impl warp::reject::Reject for Error {}

pub const COOKIE_NAME: &str = "warp-session";

pub fn login_required(
    sessions: Sessions,
) -> impl warp::Filter<Extract = (UserAccount,), Error = warp::Rejection> + Clone {
    use warp::Filter;
    warp::path::full()
        .and(warp::filters::cookie::optional(COOKIE_NAME))
        .and_then(move |path: warp::path::FullPath, cookie: Option<String>| {
            let sessions = sessions.clone();
            async move {
                get_session(sessions, &path, cookie)
                    .await
                    .and_then(|acc| {
                        acc.ok_or_else(|| Error::SessionRequired {
                            access_url: path.as_str().to_owned(),
                        })
                    })
                    .map_err(warp::reject::custom)
            }
        })
}

pub fn login_optional(
    sessions: Sessions,
) -> impl warp::Filter<Extract = (Option<UserAccount>,), Error = warp::Rejection> + Clone {
    use warp::Filter;
    warp::path::full()
        .and(warp::filters::cookie::optional(COOKIE_NAME))
        .and_then(move |path: warp::path::FullPath, cookie: Option<String>| {
            let sessions = sessions.clone();
            async move {
                get_session(sessions, &path, cookie)
                    .await
                    .map_err(warp::reject::custom)
            }
        })
}

async fn get_session(
    sessions: Sessions,
    path: &warp::path::FullPath,
    cookie: Option<String>,
) -> Result<Option<UserAccount>, Error> {
    let path = path.as_str();
    match cookie {
        Some(cookie) => {
            let session_id = Uuid::parse_str(&cookie)?;
            Ok(sessions.get_user_data(session_id).await)
        }
        None => Ok(None),
    }
    //.ok_or_else(|| Error::SessionRequired {
    //    access_url: path.to_owned(),
    //})
}

pub struct LoginSession {
    uuid: Uuid,
    expiry_time: time::Duration,
}

impl std::convert::TryFrom<LoginSession> for HeaderValue {
    type Error = std::convert::Infallible;
    fn try_from(other: LoginSession) -> Result<HeaderValue, Self::Error> {
        let cookie =
            cookie::CookieBuilder::new(crate::session::COOKIE_NAME, format!("{}", other.uuid))
                .max_age(other.expiry_time)
                .finish();
        Ok(HeaderValue::try_from(cookie.to_string()).unwrap())
    }
}

pub struct ClearCookie;

impl std::convert::TryFrom<ClearCookie> for HeaderValue {
    type Error = std::convert::Infallible;
    fn try_from(_: ClearCookie) -> Result<HeaderValue, Self::Error> {
        let cookie = cookie::CookieBuilder::new(crate::session::COOKIE_NAME, "")
            .expires(OffsetDateTime::from_unix_timestamp(0))
            .finish()
            .to_string();
        Ok(HeaderValue::try_from(cookie).unwrap())
    }
}

