use crate::user_storage::UserId;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use time::OffsetDateTime;
use tokio::{stream::StreamExt, sync::RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct Sessions(Arc<RwLock<SessionInner>>);

impl Sessions {
    pub fn new(gc_time: Duration) -> Self {
        let ret = Self(Default::default());
        let weakling = Arc::downgrade(&ret.0);
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(gc_time);
            while let Some(_) = interval.next().await {
                if let Some(this) = weakling.upgrade() {
                    let mut storage = this.write().await;
                    storage.gc();
                }
            }
        });

        ret
    }
}

impl Sessions {
    pub async fn login(&self, user_id: UserId) -> (Uuid, time::Duration) {
        let ret = Uuid::new_v4();
        let mut storage = self.0.write().await;
        let stale_session = storage
            .users_logged_in
            .get_mut(&user_id)
            .map(|stale_session| std::mem::replace(stale_session, ret));

        if let Some(stale_session) = stale_session {
            storage.sessions.remove(&stale_session);
        } else {
            storage.users_logged_in.insert(user_id, ret);
        }

        let now = time::OffsetDateTime::now_utc();

        let expiry = storage.sessions.insert(
            ret,
            SessionData {
                user_id,
                expiry: now + time::Duration::seconds(3600),
            },
        );

        (ret, time::Duration::seconds(3600))
    }

    pub async fn get_user_id(&self, session_id: Uuid) -> Option<UserId> {
        enum Id {
            Expired,
            Live(UserId),
        }

        let user_id = {
            let storage = self.0.read().await;
            let now = OffsetDateTime::now_utc();

            match storage.sessions.get(&session_id) {
                Some(data) if data.expired(now) => Some(Id::Expired),
                Some(SessionData { user_id, .. }) => Some(Id::Live(*user_id)),
                _ => None,
            }
        };

        match user_id {
            Some(Id::Live(id)) => Some(id),
            Some(Id::Expired) => {
                self.remove_session(session_id);
                None
            }
            _ => None,
        }
    }

    pub async fn remove_session(&self, session_id: Uuid) {
        self.0.write().await.remove_session(session_id);
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
            self.users_logged_in.remove(&entry.user_id);
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
    user_id: UserId,
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
}

//pub fn session_required(
//) -> impl warp::Filter<Extract = (uuid::Uuid,), Error = warp::Rejection> + Copy {
//    warp::filters::cookie::cookie("warp-session").map(|cont| {
//        let uuid = uuid::Uuid::parse_str(cont)
//            .map_err(|_| warp::reject::custom(Error::CorruptedCookie))?;
//        Ok(uuid)
//    })
//}
