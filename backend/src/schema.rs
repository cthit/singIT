// @generated automatically by Diesel CLI.

diesel::table! {
    custom_list (id) {
        id -> Int4,
        name -> Text,
    }
}

diesel::table! {
    custom_list_entry (list_id, song_hash) {
        list_id -> Int4,
        song_hash -> Text,
    }
}

diesel::table! {
    song (song_hash) {
        song_hash -> Text,
        title -> Text,
        artist -> Text,
        cover -> Nullable<Text>,
        language -> Nullable<Text>,
        video -> Nullable<Text>,
        year -> Nullable<Text>,
        genre -> Nullable<Text>,
        bpm -> Text,
        duet_singer_1 -> Nullable<Text>,
        duet_singer_2 -> Nullable<Text>,
    }
}

diesel::joinable!(custom_list_entry -> custom_list (list_id));
diesel::joinable!(custom_list_entry -> song (song_hash));

diesel::allow_tables_to_appear_in_same_query!(custom_list, custom_list_entry, song,);
