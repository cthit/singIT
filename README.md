# singIT

Karaoke song list for https://chalmers.it

## Development

- Install [Rust](https://rustup.rs/).
- Install the WASM target: `rustup target add wasm32-unknown-unknown`

Then, do the following:
```sh
# Install trunk (https://trunkrs.dev/):
cargo install --locked trunk

# Install diesel (https://diesel.rs/):
cargo install diesel_cli --no-default-features --features postgres

# Start postgres:
docker run --name "postgres" -d --publish 5432:5432 \
  --env POSTGRES_PASSWORD=password --env POSTGRES_USER=postgres postgres:16

# Configure environment variables:
cp ./backend/.env.example ./backend/.env; $EDITOR ./backend/.env

# In a dedicated terimnal: build frontend:
cd ./frontend; trunk watch

# In a dedicated terminal: configure database and run backend:
cd ./backend
diesel setup # create database and run migrations
cargo run
```

Then go to http://localhost:8080/.
There is some mock data in `./mock` that you can use to seed the database.
Good luck, have fun!
