use axum::{
    extract::{ConnectInfo, FromRef, FromRequestParts, Path, State},
    http::{
        HeaderMap,
        header::{ACCEPT_CHARSET, ACCEPT_LANGUAGE, REFERER, USER_AGENT},
        request::Parts,
    },
};
use chrono::Duration;
use rand::{Rng, rng, seq::IteratorRandom};
use sqlx::types::chrono::Utc;
use std::{
    fmt::Display,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use tokio::net::TcpStream;

use crate::{
    error::AnnounceError::{
        self, AbnormalAccess, BlacklistedClient, BlacklistedPort, DownloadPrivilegesRevoked,
        GroupNotEnabled, GroupNotFound, InfoHashNotFound, InternalTrackerError, InvalidCompact,
        InvalidDownloaded, InvalidInfoHash, InvalidLeft, InvalidNumwant, InvalidPasskey,
        InvalidPeerId, InvalidPort, InvalidQueryStringKey, InvalidQueryStringValue,
        InvalidUploaded, InvalidUserAgent, MissingDownloaded, MissingInfoHash, MissingLeft,
        MissingPeerId, MissingPort, MissingUploaded, NotAClient, PasskeyNotFound,
        PeersPerTorrentPerUserLimit, TorrentIsDeleted, TorrentIsPendingModeration,
        TorrentIsPostponed, TorrentIsRejected, TorrentNotFound, TorrentUnknownModerationStatus,
        UnsupportedEvent, UserAgentTooLong, UserNotFound,
    },
    scheduler::{
        announce_update::AnnounceUpdate,
        history_update::{self, HistoryUpdate},
        peer_update::{self, PeerUpdate},
        torrent_update::{self, TorrentUpdate},
        unregistered_info_hash_update::{self, UnregisteredInfoHashUpdate},
        user_update::{self, UserUpdate},
    },
    warning::{AnnounceWarning, WarningCollection},
};

use crate::tracker::{
    self, Tracker,
    connectable_port::ConnectablePort,
    featured_torrent::FeaturedTorrent,
    freeleech_token::FreeleechToken,
    peer::{self, Peer, PeerId},
    personal_freeleech::PersonalFreeleech,
    torrent::InfoHash,
    user::Passkey,
};
use crate::utils;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Event {
    Completed,
    #[default]
    Empty,
    Started,
    Stopped,
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::Empty => write!(f, ""),
            Self::Started => write!(f, "started"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

impl FromStr for Event {
    type Err = AnnounceError;

    fn from_str(event: &str) -> Result<Self, AnnounceError> {
        match event {
            "" | "empty" | "paused" => Ok(Self::Empty),
            "completed" => Ok(Self::Completed),
            "started" => Ok(Self::Started),
            "stopped" => Ok(Self::Stopped),
            _ => Err(UnsupportedEvent),
        }
    }
}

pub struct Announce {
    info_hash: InfoHash,
    peer_id: PeerId,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Event,
    numwant: usize,
    corrupt: Option<u64>,
    key: Option<String>,
}

pub struct Query<T>(pub T);

/// Extracts the query parameters in the HTTP GET request.
impl<S> FromRequestParts<S> for Query<Announce>
where
    S: Send + Sync,
    Arc<Tracker>: FromRef<S>,
{
    type Rejection = AnnounceError;

    async fn from_request_parts(parts: &mut Parts, tracker: &S) -> Result<Self, Self::Rejection> {
        let query_string = parts.uri.query().unwrap_or_default();
        let query_bytes = query_string.as_bytes();
        let query_length = query_bytes.len();
        let mut pos = 0;
        let mut ampersand_positions = memchr::memchr_iter(b'&', query_bytes);

        let mut info_hash: Option<InfoHash> = None;
        let mut peer_id: Option<PeerId> = None;
        let mut port: Option<u16> = None;
        let mut uploaded: Option<u64> = None;
        let mut downloaded: Option<u64> = None;
        let mut left: Option<u64> = None;
        let mut event: Option<Event> = None;
        let mut numwant: Option<usize> = None;
        let mut corrupt: Option<u64> = None;
        let mut key: Option<String> = None;

        for equal_sign_pos in memchr::memchr_iter(b'=', query_bytes) {
            let value_end_pos = ampersand_positions.next().unwrap_or(query_length);

            let parameter = query_string
                .get(pos..equal_sign_pos)
                .ok_or(InvalidQueryStringKey)?;
            let value = query_string
                .get(equal_sign_pos + 1..value_end_pos)
                .ok_or(InvalidQueryStringValue)?;

            match parameter {
                "info_hash" => {
                    info_hash = Some(InfoHash::from(
                        utils::urlencoded_to_bytes(value).or(Err(InvalidInfoHash))?,
                    ))
                }
                "peer_id" => {
                    peer_id = Some(PeerId::from(
                        utils::urlencoded_to_bytes(value).or(Err(InvalidPeerId))?,
                    ))
                }
                "port" => port = Some(value.parse().or(Err(InvalidPort))?),
                "uploaded" => uploaded = Some(value.parse().or(Err(InvalidUploaded))?),
                "downloaded" => downloaded = Some(value.parse().or(Err(InvalidDownloaded))?),
                "left" => left = Some(value.parse().or(Err(InvalidLeft))?),
                "compact" => {
                    if value != "1" {
                        return Err(InvalidCompact);
                    }
                }
                "event" => event = Some(value.parse()?),
                "numwant" => numwant = Some(value.parse().or(Err(InvalidNumwant))?),
                "corrupt" => corrupt = value.parse().ok(),
                "key" => key = value.parse().ok(),
                _ => (),
            }

            if value_end_pos == query_length {
                break;
            } else {
                pos = value_end_pos + 1;
            }
        }

        let State(tracker): State<Arc<Tracker>> = State::from_request_parts(parts, tracker)
            .await
            .or(Err(InternalTrackerError))?;

        let config = tracker.config.read();

        Ok(Query(Announce {
            info_hash: info_hash.ok_or(MissingInfoHash)?,
            peer_id: peer_id.ok_or(MissingPeerId)?,
            port: port.ok_or(MissingPort)?,
            uploaded: uploaded.ok_or(MissingUploaded)?,
            downloaded: downloaded.ok_or(MissingDownloaded)?,
            left: left.ok_or(MissingLeft)?,
            event: event.unwrap_or_default(),
            numwant: {
                if event.unwrap_or_default() == Event::Stopped {
                    0
                } else {
                    numwant
                        .unwrap_or(config.numwant_default)
                        .min(config.numwant_max)
                }
            },
            corrupt,
            key,
        }))
    }
}

pub struct ClientIp(pub std::net::IpAddr);

impl FromRequestParts<Arc<Tracker>> for ClientIp {
    type Rejection = AnnounceError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<Tracker>,
    ) -> Result<Self, Self::Rejection> {
        let header_name_opt = &state
            .config
            .read()
            .reverse_proxy_client_ip_header_name
            .to_owned();

        let ip = if let Some(header) = &header_name_opt {
            // We need the right-most ip, which should be included in the last header
            parts
                .headers
                .get_all(header)
                .iter()
                .last()
                // Err: Missing the client ip header
                .ok_or(InternalTrackerError)?
                .to_str()
                // Err: Client ip header is not UTF-8
                .map_err(|_| InternalTrackerError)?
                .split(',')
                .last()
                // Err: Client ip header is empty
                .ok_or(InternalTrackerError)?
                .trim()
                .parse()
                // Err: Client ip header does not contain a valid ip address
                .map_err(|_| InternalTrackerError)?
        } else {
            // If the header isn't configured, use the connecting ip.
            let ConnectInfo(addr) = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
                .await
                .or(Err(InternalTrackerError))?;

            addr.ip()
        };

        Ok(ClientIp(ip))
    }
}

pub async fn announce(
    State(tracker): State<Arc<Tracker>>,
    Path(passkey): Path<String>,
    Query(queries): Query<Announce>,
    headers: HeaderMap,
    ClientIp(client_ip): ClientIp,
) -> Result<Vec<u8>, AnnounceError> {
    // Validate headers
    if headers.contains_key(ACCEPT_LANGUAGE)
        || headers.contains_key(REFERER)
        || headers.contains_key(ACCEPT_CHARSET)
        // This header check may block Non-bittorrent client `Aria2` to access tracker,
        // Because they always add this header which other clients don't have.
        //
        // See: https://blog.rhilip.info/archives/1010/ ( in Chinese )
        || headers.contains_key("want-digest")
    {
        return Err(AbnormalAccess);
    }

    // User agent header is required.
    let user_agent = headers
        .get(USER_AGENT)
        .ok_or(InvalidUserAgent)?
        .to_str()
        .or(Err(InvalidUserAgent))?;

    // Block user agent strings that are too long. (For Database reasons)
    if user_agent.len() > 64 {
        return Err(UserAgentTooLong);
    }

    // Block user agent strings on the blacklist
    for client in tracker.agent_blacklist.read().iter() {
        if queries.peer_id.starts_with(&client.peer_id_prefix) {
            return Err(BlacklistedClient);
        }
    }

    // Block user agent strings on the regex blacklist
    let user_agent_lower = user_agent.to_ascii_lowercase();

    if user_agent_lower.contains("mozilla")
        || user_agent_lower.contains("browser")
        || user_agent_lower.contains("chrome")
        || user_agent_lower.contains("safari")
        || user_agent_lower.contains("applewebkit")
        || user_agent_lower.contains("opera")
        || user_agent_lower.contains("links")
        || user_agent_lower.contains("lynx")
        || user_agent_lower.contains("bot")
        || user_agent_lower.contains("unknown")
    {
        return Err(NotAClient);
    }

    // Validate port
    // Some clients send port 0 on the stopped event
    if tracker.port_blacklist.read().contains(&queries.port) && queries.event != Event::Stopped {
        return Err(BlacklistedPort(queries.port));
    }

    let passkey: Passkey = Passkey::from_str(&passkey).or(Err(InvalidPasskey))?;

    // Validate passkey
    let user_id = tracker
        .passkey2id
        .read()
        .get(&passkey)
        .ok_or(PasskeyNotFound)
        .cloned();

    let user = if let Ok(user_id) = user_id {
        tracker
            .users
            .read()
            .get(&user_id)
            .ok_or(UserNotFound)
            .cloned()
    } else {
        Err(UserNotFound)
    };

    // Validate torrent
    let torrent_id_res = tracker
        .infohash2id
        .read()
        .get(&queries.info_hash)
        .ok_or(InfoHashNotFound)
        .cloned();

    let now = Utc::now();

    if let Ok(user) = &user {
        if let Err(InfoHashNotFound) = torrent_id_res {
            tracker.unregistered_info_hash_updates.lock().upsert(
                unregistered_info_hash_update::Index {
                    user_id: user.id,
                    info_hash: queries.info_hash,
                },
                UnregisteredInfoHashUpdate {
                    created_at: now,
                    updated_at: now,
                },
            );
        }
    }

    let torrent_id = torrent_id_res?;

    let is_connectable = check_connectivity(&tracker, client_ip, queries.port).await;

    let mut warnings = WarningCollection::new();

    let config = tracker.config.read();

    if !is_connectable && config.require_peer_connectivity {
        warnings.add(AnnounceWarning::ConnectivityIssueDetected);
    }

    let (
        upload_factor,
        download_factor,
        uploaded_delta,
        downloaded_delta,
        seeder_delta,
        leecher_delta,
        times_completed_delta,
        is_visible,
        user,
        user_id,
        group,
        has_requested_seed_list,
        has_requested_leech_list,
        should_early_return,
        response,
    ) = {
        let mut torrent_guard = tracker.torrents.lock();
        let torrent = torrent_guard.get_mut(&torrent_id).ok_or(TorrentNotFound)?;

        if torrent.is_deleted {
            return Err(TorrentIsDeleted);
        }

        match torrent.status {
            tracker::torrent::Status::Approved => (),
            tracker::torrent::Status::Pending => return Err(TorrentIsPendingModeration),
            tracker::torrent::Status::Rejected => return Err(TorrentIsRejected),
            tracker::torrent::Status::Postponed => return Err(TorrentIsPostponed),
            _ => return Err(TorrentUnknownModerationStatus),
        }

        let user_id = user_id?;
        let user = user?;

        // Validate user
        if !user.can_download && queries.left != 0 {
            return Err(DownloadPrivilegesRevoked);
        }

        let group = tracker
            .groups
            .read()
            .get(&user.group_id)
            .ok_or(GroupNotFound)?
            .clone();

        if ["banned", "validating", "disabled"].contains(&group.slug.as_str()) {
            return Err(GroupNotEnabled(group.slug));
        }

        // Make sure user isn't leeching more torrents than their group allows
        let has_hit_download_slot_limit = if queries.left > 0 {
            if let Some(slots) = group.download_slots {
                user.num_leeching >= slots
            } else {
                false
            }
        } else {
            false
        };

        // Change of upload/download compared to previous announce
        let uploaded_delta;
        let downloaded_delta;
        let seeder_delta;
        let leecher_delta;
        let times_completed_delta;
        let is_visible;

        if queries.event == Event::Stopped {
            // Try and remove the peer
            let removed_peer = torrent.peers.swap_remove(&tracker::peer::Index {
                user_id,
                peer_id: queries.peer_id,
            });
            // Check if peer was removed
            if let Some(peer) = removed_peer {
                // Calculate change in upload and download compared to previous
                // announce
                uploaded_delta = queries.uploaded.saturating_sub(peer.uploaded);
                downloaded_delta = queries.downloaded.saturating_sub(peer.downloaded);

                leecher_delta = 0 - peer.is_included_in_leech_list(&config) as i32;
                seeder_delta = 0 - peer.is_included_in_seed_list(&config) as i32;
            } else {
                // Some clients (namely transmission) will keep sending
                // `stopped` events until a successful announce is received.
                // If a user's network is having issues, their peer might be
                // deleted for inactivity from missed announces. If their peer
                // isn't found when we receive a `stopped` event from them
                // after regaining network connectivity, we can't return an
                // error otherwise the client might enter into an infinite loop
                // of sending `stopped` events. To prevent this, we need to
                // send a warning (i.e. succcessful announce) instead, so that
                // the client can successfully restart its session.
                warnings.add(AnnounceWarning::StoppedPeerDoesntExist);
                leecher_delta = 0;
                seeder_delta = 0;
                uploaded_delta = 0;
                downloaded_delta = 0;
            }

            times_completed_delta = 0;
            is_visible = false;
        } else {
            // Insert the peer into the in-memory db
            let mut old_peer: Option<Peer> = None;
            let new_peer = *torrent
                .peers
                .entry(tracker::peer::Index {
                    user_id,
                    peer_id: queries.peer_id,
                })
                .and_modify(|peer| {
                    old_peer = Some(*peer);

                    peer.ip_address = client_ip;
                    peer.port = queries.port;
                    peer.is_seeder = queries.left == 0;
                    peer.is_connectable = is_connectable;
                    peer.is_visible =
                        peer.is_included_in_leech_list(&config) || !has_hit_download_slot_limit;
                    peer.is_active = true;
                    peer.updated_at = now;
                    peer.uploaded = queries.uploaded;
                    peer.downloaded = queries.downloaded;
                })
                .or_insert(tracker::Peer {
                    ip_address: client_ip,
                    port: queries.port,
                    is_seeder: queries.left == 0,
                    is_active: true,
                    is_visible: !has_hit_download_slot_limit,
                    is_connectable,
                    updated_at: now,
                    uploaded: queries.uploaded,
                    downloaded: queries.downloaded,
                });

            is_visible = new_peer.is_visible;

            // Warn user if download slots are full
            if !is_visible {
                warnings.add(AnnounceWarning::HitDownloadSlotLimit);
            };

            // Update the user and torrent seeding/leeching counts in the
            // in-memory db
            match old_peer {
                Some(old_peer) => {
                    leecher_delta = new_peer.is_included_in_leech_list(&config) as i32
                        - old_peer.is_included_in_leech_list(&config) as i32;
                    seeder_delta = new_peer.is_included_in_seed_list(&config) as i32
                        - old_peer.is_included_in_seed_list(&config) as i32;
                    times_completed_delta = (new_peer.is_seeder && !old_peer.is_seeder) as u32;

                    // Calculate change in upload and download compared to previous
                    // announce
                    if queries.uploaded < old_peer.uploaded
                        || queries.downloaded < old_peer.downloaded
                    {
                        // Client sent the same peer id but restarted the session
                        // Assume delta is 0
                        uploaded_delta = 0;
                        downloaded_delta = 0;
                    } else {
                        // Assume client continues previously tracked session
                        uploaded_delta = queries.uploaded - old_peer.uploaded;
                        downloaded_delta = queries.downloaded - old_peer.downloaded;
                    }

                    // Warn user if peer last announced less than
                    // announce_min_enforced seconds ago
                    if old_peer
                        .updated_at
                        .checked_add_signed(Duration::seconds(config.announce_min_enforced.into()))
                        .is_some_and(|blocked_until| blocked_until > now)
                    {
                        warnings.add(AnnounceWarning::RateLimitExceeded);
                    }
                }
                None => {
                    // new peer is inserted

                    // Make sure user is only allowed N peers per torrent.
                    let mut peer_count = 0;

                    for (&index, &peer) in torrent.peers.iter() {
                        if index.user_id == user_id && peer.is_active {
                            peer_count += 1;

                            if peer_count > config.max_peers_per_torrent_per_user {
                                torrent.peers.swap_remove(&tracker::peer::Index {
                                    user_id,
                                    peer_id: queries.peer_id,
                                });

                                return Err(PeersPerTorrentPerUserLimit(
                                    config.max_peers_per_torrent_per_user,
                                ));
                            }
                        }
                    }

                    leecher_delta = new_peer.is_included_in_leech_list(&config) as i32;
                    seeder_delta = new_peer.is_included_in_seed_list(&config) as i32;
                    times_completed_delta = 0;

                    // Calculate change in upload and download compared to previous
                    // announce
                    uploaded_delta = 0;
                    downloaded_delta = 0;
                }
            }
        }

        // Compute this before we convert the warnings into a message.
        let should_early_return = warnings.should_early_return();

        // Has to be adjusted before the peer list is generated
        torrent.seeders = torrent.seeders.saturating_add_signed(seeder_delta);
        torrent.leechers = torrent.leechers.saturating_add_signed(leecher_delta);
        torrent.times_completed = torrent
            .times_completed
            .saturating_add(times_completed_delta);

        // Generate peer lists to return to client

        let mut peers_ipv4: Vec<u8> = Vec::new();
        let mut peers_ipv6: Vec<u8> = Vec::new();

        let mut has_requested_seed_list = false;
        let mut has_requested_leech_list = false;
        let mut is_over_seed_list_rate_limit = false;
        let mut is_over_leech_list_rate_limit = false;

        // Only provide peer list if
        // - it is not a stopped event,
        // - there exist leechers (we have to remember to update the torrent leecher count before this check)
        // - there is no warning in the response
        if queries.event != Event::Stopped && torrent.leechers > 0 && warnings.is_empty() {
            let mut peers: Vec<(&peer::Index, &Peer)> = Vec::with_capacity(std::cmp::min(
                queries.numwant,
                torrent.seeders as usize + torrent.leechers as usize,
            ));

            // Don't return peers with the same user id or those that are marked as inactive
            let valid_peers = torrent.peers.iter().filter(|(index, peer)| {
                index.user_id != user_id && peer.is_included_in_peer_list(&config)
            });

            // Make sure leech peer lists are filled with seeds
            if queries.left > 0 && torrent.seeders > 0 && queries.numwant > peers.len() {
                has_requested_seed_list = true;

                if user.receive_seed_list_rates.is_under_limit() {
                    peers.extend(
                        valid_peers
                            .clone()
                            .filter(|(_index, peer)| peer.is_seeder)
                            .choose_multiple(&mut rng(), queries.numwant),
                    );
                } else {
                    is_over_seed_list_rate_limit = true;
                }
            }

            // Otherwise only send leeches until the numwant is reached
            if torrent.leechers > 0 && queries.numwant > peers.len() {
                has_requested_leech_list = true;

                if user.receive_leech_list_rates.is_under_limit() {
                    peers.extend(
                        valid_peers
                            .filter(|(_index, peer)| !peer.is_seeder)
                            .choose_multiple(
                                &mut rng(),
                                queries.numwant.saturating_sub(peers.len()),
                            ),
                    );
                } else {
                    is_over_leech_list_rate_limit = true;
                }
            }

            // Split peers into ipv4 and ipv6 variants and serialize their socket
            // to bytes according to the bittorrent spec
            for (_index, peer) in peers.iter() {
                match peer.ip_address {
                    IpAddr::V4(ip) => {
                        peers_ipv4.extend(&ip.octets());
                        peers_ipv4.extend(&peer.port.to_be_bytes());
                    }
                    IpAddr::V6(ip) => {
                        peers_ipv6.extend(&ip.octets());
                        peers_ipv6.extend(&peer.port.to_be_bytes());
                    }
                }
            }
        }

        // Generate bencoded response to return to client

        let interval = rng().random_range(config.announce_min..=config.announce_max);

        // Write out bencoded response (keys must be sorted to be within spec)
        let mut response: Vec<u8> = Vec::with_capacity(
            82 // literal characters
                + 5 * 5 // numbers with estimated digit quantity for each
                + peers_ipv4.len() * 6 + 5 // bytes per ipv4 plus estimated length prefix
                + peers_ipv6.len() * 18 + 5 // bytes per ipv6 plus estimated length prefix
                + warnings.max_byte_length(), // max bytes per warning message plus separator
        );

        response.extend(b"d8:completei");

        if is_over_seed_list_rate_limit || !warnings.is_empty() {
            response.extend(b"0")
        } else {
            response.extend(torrent.seeders.to_string().as_bytes());
        }

        response.extend(b"e10:downloadedi");
        response.extend(torrent.times_completed.to_string().as_bytes());
        response.extend(b"e10:incompletei");

        if is_over_leech_list_rate_limit || !warnings.is_empty() {
            response.extend(b"0");
        } else {
            response.extend(torrent.leechers.to_string().as_bytes());
        }

        response.extend(b"e8:intervali");
        response.extend(interval.to_string().as_bytes());
        response.extend(b"e12:min intervali");
        response.extend(config.announce_min.to_string().as_bytes());
        response.extend(b"e5:peers");

        if peers_ipv4.is_empty() {
            response.extend(b"0:")
        } else {
            response.extend(peers_ipv4.len().to_string().as_bytes());
            response.extend(b":");
            response.extend(&peers_ipv4);
        }

        if !peers_ipv6.is_empty() {
            response.extend(b"6:peers6");
            response.extend(peers_ipv6.len().to_string().as_bytes());
            response.extend(b":");
            response.extend(peers_ipv6);
        }

        if let Some(warning_message) = warnings.into_message() {
            response.extend(b"15:warning message");
            response.extend(warning_message.len().to_string().as_bytes());
            response.extend(b":");
            response.extend(warning_message);
        }

        response.extend(b"e");

        let mut upload_factor = std::cmp::max(
            config.upload_factor,
            std::cmp::max(group.upload_factor, torrent.upload_factor),
        );

        let mut download_factor = std::cmp::min(
            config.download_factor,
            std::cmp::min(group.download_factor, torrent.download_factor),
        );

        if user.is_lifetime {
            if let Some(override_upload_factor) = config.lifetime_donor_upload_factor_override {
                upload_factor = std::cmp::max(upload_factor, override_upload_factor)
            }

            if let Some(override_download_factor) = config.lifetime_donor_download_factor_override {
                download_factor = std::cmp::min(download_factor, override_download_factor)
            }
        } else if user.is_donor {
            if let Some(override_upload_factor) = config.donor_upload_factor_override {
                upload_factor = std::cmp::max(upload_factor, override_upload_factor)
            }

            if let Some(override_download_factor) = config.donor_download_factor_override {
                download_factor = std::cmp::min(download_factor, override_download_factor)
            }
        }

        // Has to be dropped before any `await` calls.
        //
        // Unfortunately, `Drop` currently doesn't work in rust with borrowed values
        // so we have to use a giant scope instead.
        //
        // See:
        // - https://github.com/rust-lang/rust/issues/57478
        // - https://stackoverflow.com/questions/73519148/why-does-send-value-that-is-dropd-before-await-mean-the-future-is-send
        // - https://github.com/rust-lang/rust/issues/101135
        //    - Once this issue is fixed, we can remove the scope and rely solely on `Drop`.
        drop(torrent_guard);

        (
            upload_factor,
            download_factor,
            uploaded_delta,
            downloaded_delta,
            seeder_delta,
            leecher_delta,
            times_completed_delta,
            is_visible,
            user,
            user_id,
            group,
            has_requested_seed_list,
            has_requested_leech_list,
            should_early_return,
            response,
        )
    };

    // Short circuit response for stopped peer doesn't exist error since we
    // can't do anything with it and don't want to update any data.
    if should_early_return {
        return Ok(response);
    }

    let download_factor = if tracker
        .personal_freeleeches
        .read()
        .contains(&PersonalFreeleech { user_id })
        || tracker.freeleech_tokens.read().contains(&FreeleechToken {
            user_id,
            torrent_id,
        })
        || tracker
            .featured_torrents
            .read()
            .contains(&FeaturedTorrent { torrent_id })
    {
        0
    } else {
        download_factor
    };

    let upload_factor = if tracker
        .featured_torrents
        .read()
        .contains(&FeaturedTorrent { torrent_id })
    {
        200
    } else {
        upload_factor
    };

    let credited_uploaded_delta = upload_factor as u64 * uploaded_delta / 100;
    let credited_downloaded_delta = download_factor as u64 * downloaded_delta / 100;

    let completed_at = if queries.event == Event::Completed {
        Some(now)
    } else {
        None
    };

    if seeder_delta != 0
        || leecher_delta != 0
        || has_requested_seed_list
        || has_requested_leech_list
    {
        tracker.users.write().entry(user_id).and_modify(|user| {
            user.num_seeding = user.num_seeding.saturating_add_signed(seeder_delta);
            user.num_leeching = user.num_leeching.saturating_add_signed(leecher_delta);

            if has_requested_seed_list {
                user.receive_seed_list_rates.tick();
            }

            if has_requested_leech_list {
                user.receive_leech_list_rates.tick();
            }
        });
    }

    tracker.peer_updates.lock().upsert(
        peer_update::Index {
            peer_id: queries.peer_id,
            torrent_id,
            user_id,
        },
        PeerUpdate {
            ip: client_ip,
            port: queries.port,
            agent: String::from(user_agent),
            uploaded: queries.uploaded,
            downloaded: queries.downloaded,
            is_active: queries.event != Event::Stopped,
            is_seeder: queries.left == 0,
            is_visible,
            left: queries.left,
            created_at: now,
            updated_at: now,
            connectable: is_connectable,
        },
    );

    tracker.history_updates.lock().upsert(
        history_update::Index {
            user_id,
            torrent_id,
        },
        HistoryUpdate {
            user_agent: String::from(user_agent),
            is_active: queries.event != Event::Stopped,
            is_seeder: queries.left == 0,
            is_immune: if user.is_lifetime {
                config
                    .lifetime_donor_immunity_override
                    .unwrap_or(group.is_immune)
            } else if user.is_donor {
                config.donor_immunity_override.unwrap_or(group.is_immune)
            } else {
                group.is_immune
            },
            uploaded: queries.uploaded,
            downloaded: queries.downloaded,
            uploaded_delta,
            downloaded_delta,
            credited_uploaded_delta,
            credited_downloaded_delta,
            completed_at,
            created_at: now,
            updated_at: now,
        },
    );

    if credited_uploaded_delta != 0 || credited_downloaded_delta != 0 {
        tracker.user_updates.lock().upsert(
            user_update::Index { user_id },
            UserUpdate {
                uploaded_delta: credited_uploaded_delta,
                downloaded_delta: credited_downloaded_delta,
            },
        );
    }

    if seeder_delta != 0
        || leecher_delta != 0
        || times_completed_delta != 0
        || uploaded_delta != 0
        || downloaded_delta != 0
    {
        tracker.torrent_updates.lock().upsert(
            torrent_update::Index { torrent_id },
            TorrentUpdate {
                seeder_delta,
                leecher_delta,
                times_completed_delta,
                balance_delta: uploaded_delta.try_into().unwrap_or(i64::MAX)
                    - downloaded_delta.try_into().unwrap_or(i64::MAX),
            },
        );
    }

    if config.is_announce_logging_enabled {
        tracker.announce_updates.lock().upsert(AnnounceUpdate {
            user_id,
            torrent_id,
            uploaded: queries.uploaded,
            downloaded: queries.downloaded,
            left: queries.left,
            corrupt: queries.corrupt,
            peer_id: queries.peer_id,
            port: queries.port,
            numwant: queries.numwant.try_into().unwrap_or(u16::MAX),
            created_at: now,
            event: queries.event,
            key: queries.key,
        });
    }

    tracker.stats.increment_announce_response();

    Ok(response)
}

async fn check_connectivity(tracker: &Arc<Tracker>, ip: IpAddr, port: u16) -> bool {
    if tracker.config.read().is_connectivity_check_enabled {
        let now = Utc::now();
        let socket = SocketAddr::from((ip, port));
        let connectable_port_opt = tracker.connectable_ports.read().get(&socket).cloned();

        if let Some(connectable_port) = connectable_port_opt {
            let ttl = Duration::seconds(tracker.config.read().connectivity_check_interval);

            if let Some(cached_until) = connectable_port.updated_at.checked_add_signed(ttl) {
                if cached_until > now {
                    return connectable_port.connectable;
                }
            }
        }

        let connectable = tokio::spawn(async move {
            tokio::time::timeout(
                std::time::Duration::from_millis(500),
                TcpStream::connect(socket),
            )
            .await
            .is_ok_and(|connection_result| connection_result.is_ok())
        })
        .await
        .unwrap_or(false);

        tracker
            .connectable_ports
            .write()
            .entry(socket)
            .and_modify(|connectable_port| {
                connectable_port.connectable = connectable;
                connectable_port.updated_at = now;
            })
            .or_insert(ConnectablePort {
                connectable,
                updated_at: now,
            });

        return connectable;
    }

    false
}
