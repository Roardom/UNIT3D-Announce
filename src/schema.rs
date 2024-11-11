// @generated automatically by Diesel CLI.

diesel::table! {
    announces (id) {
        id -> Unsigned<Bigint>,
        user_id -> Unsigned<Integer>,
        torrent_id -> Unsigned<Integer>,
        uploaded -> Unsigned<Bigint>,
        downloaded -> Unsigned<Bigint>,
        left -> Unsigned<Bigint>,
        corrupt -> Unsigned<Bigint>,
        #[max_length = 20]
        peer_id -> Binary,
        port -> Unsigned<Smallint>,
        numwant -> Unsigned<Smallint>,
        created_at -> Timestamp,
        #[max_length = 255]
        event -> Varchar,
        #[max_length = 255]
        key -> Varchar,
    }
}

diesel::table! {
    blacklist_clients (id) {
        id -> Unsigned<Bigint>,
        #[max_length = 255]
        name -> Varchar,
        reason -> Nullable<Longtext>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 255]
        peer_id_prefix -> Varchar,
    }
}

diesel::table! {
    featured_torrents (id) {
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        torrent_id -> Unsigned<Integer>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    freeleech_tokens (id) {
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        torrent_id -> Unsigned<Integer>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    groups (id) {
        id -> Integer,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        slug -> Varchar,
        position -> Integer,
        level -> Integer,
        download_slots -> Nullable<Integer>,
        #[max_length = 255]
        color -> Varchar,
        #[max_length = 255]
        icon -> Varchar,
        #[max_length = 255]
        effect -> Varchar,
        is_internal -> Bool,
        is_editor -> Bool,
        is_owner -> Bool,
        is_admin -> Bool,
        is_modo -> Bool,
        is_trusted -> Bool,
        is_immune -> Bool,
        is_freeleech -> Bool,
        is_double_upload -> Bool,
        is_refundable -> Bool,
        can_upload -> Bool,
        is_incognito -> Bool,
        autogroup -> Bool,
        min_uploaded -> Nullable<Unsigned<Bigint>>,
        min_seedsize -> Nullable<Unsigned<Bigint>>,
        min_avg_seedtime -> Nullable<Unsigned<Bigint>>,
        min_ratio -> Nullable<Decimal>,
        min_age -> Nullable<Unsigned<Bigint>>,
        system_required -> Bool,
    }
}

diesel::table! {
    history (id) {
        id -> Unsigned<Bigint>,
        user_id -> Unsigned<Integer>,
        torrent_id -> Unsigned<Integer>,
        #[max_length = 64]
        agent -> Varchar,
        uploaded -> Unsigned<Bigint>,
        actual_uploaded -> Unsigned<Bigint>,
        client_uploaded -> Unsigned<Bigint>,
        downloaded -> Unsigned<Bigint>,
        refunded_download -> Unsigned<Bigint>,
        actual_downloaded -> Unsigned<Bigint>,
        client_downloaded -> Unsigned<Bigint>,
        seeder -> Bool,
        active -> Bool,
        seedtime -> Unsigned<Bigint>,
        immune -> Bool,
        hitrun -> Bool,
        prewarn -> Bool,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        completed_at -> Nullable<Datetime>,
    }
}

diesel::table! {
    peers (id) {
        id -> Unsigned<Bigint>,
        #[max_length = 20]
        peer_id -> Binary,
        #[max_length = 16]
        ip -> Varbinary,
        port -> Unsigned<Smallint>,
        #[max_length = 64]
        agent -> Varchar,
        uploaded -> Unsigned<Bigint>,
        downloaded -> Unsigned<Bigint>,
        left -> Unsigned<Bigint>,
        seeder -> Bool,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        torrent_id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        connectable -> Bool,
        active -> Bool,
        visible -> Bool,
    }
}

diesel::table! {
    personal_freeleeches (id) {
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    torrents (id) {
        id -> Unsigned<Integer>,
        #[max_length = 255]
        name -> Varchar,
        description -> Text,
        mediainfo -> Nullable<Text>,
        bdinfo -> Nullable<Longtext>,
        #[max_length = 255]
        file_name -> Varchar,
        num_file -> Integer,
        #[max_length = 255]
        folder -> Nullable<Varchar>,
        size -> Double,
        nfo -> Nullable<Blob>,
        leechers -> Integer,
        seeders -> Integer,
        times_completed -> Integer,
        category_id -> Nullable<Integer>,
        user_id -> Unsigned<Integer>,
        imdb -> Unsigned<Integer>,
        tvdb -> Unsigned<Integer>,
        tmdb -> Unsigned<Integer>,
        mal -> Unsigned<Integer>,
        #[max_length = 255]
        igdb -> Varchar,
        season_number -> Nullable<Integer>,
        episode_number -> Nullable<Integer>,
        stream -> Bool,
        free -> Smallint,
        doubleup -> Bool,
        refundable -> Bool,
        highspeed -> Bool,
        featured -> Bool,
        status -> Smallint,
        moderated_at -> Nullable<Datetime>,
        moderated_by -> Nullable<Integer>,
        anon -> Smallint,
        sticky -> Smallint,
        sd -> Bool,
        internal -> Bool,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        bumped_at -> Nullable<Datetime>,
        fl_until -> Nullable<Datetime>,
        du_until -> Nullable<Datetime>,
        release_year -> Nullable<Unsigned<Smallint>>,
        deleted_at -> Nullable<Timestamp>,
        type_id -> Integer,
        resolution_id -> Nullable<Integer>,
        distributor_id -> Nullable<Integer>,
        region_id -> Nullable<Integer>,
        personal_release -> Integer,
        balance -> Nullable<Bigint>,
        balance_offset -> Nullable<Bigint>,
        #[max_length = 20]
        info_hash -> Binary,
    }
}

diesel::table! {
    users (id) {
        id -> Unsigned<Integer>,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password -> Varchar,
        two_factor_secret -> Nullable<Text>,
        two_factor_recovery_codes -> Nullable<Text>,
        two_factor_confirmed_at -> Nullable<Timestamp>,
        #[max_length = 255]
        passkey -> Varchar,
        group_id -> Integer,
        internal_id -> Nullable<Integer>,
        active -> Bool,
        uploaded -> Unsigned<Bigint>,
        downloaded -> Unsigned<Bigint>,
        #[max_length = 255]
        image -> Nullable<Varchar>,
        #[max_length = 255]
        title -> Nullable<Varchar>,
        about -> Nullable<Mediumtext>,
        signature -> Nullable<Text>,
        fl_tokens -> Unsigned<Integer>,
        seedbonus -> Decimal,
        invites -> Unsigned<Integer>,
        hitandruns -> Unsigned<Integer>,
        #[max_length = 255]
        rsskey -> Varchar,
        chatroom_id -> Unsigned<Integer>,
        censor -> Bool,
        chat_hidden -> Bool,
        hidden -> Bool,
        style -> Bool,
        torrent_layout -> Bool,
        torrent_filters -> Bool,
        #[max_length = 255]
        custom_css -> Nullable<Varchar>,
        #[max_length = 255]
        standalone_css -> Nullable<Varchar>,
        read_rules -> Bool,
        can_chat -> Bool,
        can_comment -> Bool,
        can_download -> Bool,
        can_request -> Bool,
        can_invite -> Bool,
        can_upload -> Bool,
        show_poster -> Bool,
        peer_hidden -> Bool,
        private_profile -> Bool,
        block_notifications -> Bool,
        stat_hidden -> Bool,
        #[max_length = 255]
        remember_token -> Nullable<Varchar>,
        #[max_length = 100]
        api_token -> Nullable<Varchar>,
        last_login -> Nullable<Datetime>,
        last_action -> Nullable<Datetime>,
        disabled_at -> Nullable<Datetime>,
        deleted_by -> Nullable<Unsigned<Integer>>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 255]
        locale -> Varchar,
        chat_status_id -> Unsigned<Integer>,
        deleted_at -> Nullable<Timestamp>,
        own_flushes -> Tinyint,
        email_verified_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(featured_torrents -> torrents (torrent_id));
diesel::joinable!(featured_torrents -> users (user_id));
diesel::joinable!(freeleech_tokens -> torrents (torrent_id));
diesel::joinable!(freeleech_tokens -> users (user_id));
diesel::joinable!(history -> torrents (torrent_id));
diesel::joinable!(history -> users (user_id));
diesel::joinable!(peers -> torrents (torrent_id));
diesel::joinable!(peers -> users (user_id));
diesel::joinable!(personal_freeleeches -> users (user_id));
diesel::joinable!(torrents -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    announces,
    blacklist_clients,
    featured_torrents,
    freeleech_tokens,
    groups,
    history,
    peers,
    personal_freeleeches,
    torrents,
    users,
);
