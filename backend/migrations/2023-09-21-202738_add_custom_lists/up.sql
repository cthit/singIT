CREATE TABLE custom_list (
	id SERIAL PRIMARY KEY,
	name TEXT UNIQUE NOT NULL
);

CREATE TABLE custom_list_entry (
	list_id SERIAL REFERENCES custom_list(id),
	song_hash TEXT NOT NULL REFERENCES song(song_hash),
	PRIMARY KEY (list_id, song_hash)
);
