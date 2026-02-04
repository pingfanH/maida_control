create table achievements
(
    id           bigint auto_increment
        primary key,
    song_id      int    not null,
    uid          bigint not null,
    level_index  int    not null,
    achievements int    not null,
    constant     float  not null,
    dx_score     int    not null,
    fc           text   not null,
    fs           text   not null,
    ra           int    not null,
    constraint achievements_pk
        unique (song_id, uid, level_index)
);

create table chart
(
    id          int   not null
        primary key,
    ds          float not null,
    music_id    int   not null,
    level       text  not null,
    level_index int   not null,
    charter     text  not null,
    tap         int   not null,
    hold        int   not null,
    slide       int   not null,
    break_note  int   not null,
    touch       int   not null
);

create table music_data
(
    id             int  not null
        primary key,
    title          text not null,
    type_field     text not null,
    alias          text null,
    addition_alias text null,
    artist         text not null,
    genre          text not null,
    bpm            int  not null,
    `from`         text not null,
    flag           int  not null
)
    collate = utf8mb4_unicode_ci;

create table music_alias
(
    id       bigint auto_increment
        primary key,
    song_id  int  not null,
    alias    varchar(255) not null,
    constraint music_alias_song_id_alias_uindex
        unique (song_id, alias)
);

create table user_favorite_music
(
    id           bigint auto_increment
        primary key,
    open_user_id varchar(64) not null,
    title      varchar(50)         not null,
    create_time  datetime default CURRENT_TIMESTAMP null,
    update_time  datetime default CURRENT_TIMESTAMP null on update CURRENT_TIMESTAMP,
    constraint user_favorite_music_uindex
        unique (open_user_id, song_id)
);

create table user
(
    id            bigint auto_increment
        primary key,
    qq            bigint                             not null,
    user_name     text                               not null,
    qq_name       text                               not null,
    rating        int                                not null,
    mai_id        int                                null,
    create_time   datetime default CURRENT_TIMESTAMP null,
    update_time   datetime default CURRENT_TIMESTAMP null on update CURRENT_TIMESTAMP,
    group_list    text                               null,
    fish_token    text                               null,
    fish_password text                               null,
    avatar        int                                null,
    name_plate    int                                null,
    constraint user_pk
        unique (qq)
);

create table maimai_session
(
    id               bigint auto_increment
        primary key,
    user_id          bigint      not null,
    open_user_id     varchar(64) not null,
    session_id       bigint      not null,
    open_game_id     varchar(64) not null,
    user_play_flag   tinyint     not null,
    new_user_id_flag tinyint     not null,
    open_game_id_flag tinyint    not null,
    create_time      datetime default CURRENT_TIMESTAMP null,
    update_time      datetime default CURRENT_TIMESTAMP null on update CURRENT_TIMESTAMP,
    constraint maimai_session_user_id_uindex
        unique (user_id)
);

create table maimai_cookie
(
    id           bigint auto_increment
        primary key,
    open_user_id varchar(64) not null,
    cookies_json longtext    not null,
    create_time  datetime default CURRENT_TIMESTAMP null,
    update_time  datetime default CURRENT_TIMESTAMP null on update CURRENT_TIMESTAMP,
    constraint maimai_cookie_open_user_id_uindex
        unique (open_user_id)
);

create table maimai_cache
(
    id           bigint auto_increment
        primary key,
    cache_key    varchar(64) not null,
    open_user_id varchar(64) not null,
    content      longtext    not null,
    create_time  datetime default CURRENT_TIMESTAMP null,
    update_time  datetime default CURRENT_TIMESTAMP null on update CURRENT_TIMESTAMP,
    constraint maimai_cache_key_user_uindex
        unique (cache_key, open_user_id)
);
