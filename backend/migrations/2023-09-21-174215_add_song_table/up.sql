CREATE TABLE song(
    song_hash TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    cover TEXT,
    language TEXT,
    video TEXT,
    year TEXT,
    genre TEXT,
    bpm TEXT NOT NULL,
    duet_singer_1 TEXT,
    duet_singer_2 TEXT
);
