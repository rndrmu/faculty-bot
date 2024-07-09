# Faculty bot (Discord)

## Introduction

This project features a Discord bot with the intent to reduce the administration overhead for faculty related tasks. The bot takes care of verifying new server members.

## Setting up the bot

- To run the code itself, first you need [The Rust Programming Language](https://rust-lang.org). Install this using:
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  and choose the `stable` toolchain.

- Download the project and execute:
  ```sh
  cargo build --release
  ```
  inside the primary folder using Windows Terminal or any zsh/bash on *nix systems. This will compile a release-optimized build of the bot.

- Create a `.env` file with the following content:
  ```sh
  DISCORD_TOKEN=

  # PostgreSQL format e.g. postgres://dbuser:dbpass@localhost:5432/db_name
  DATABASE_URL=

  PREFIX=

  MAILUSER=
  MAILPW=
  SMTP_SERVER=
  SMTP_PORT=

  POSTGRES_USER=
  POSTGRES_PASSWORD=
  POSTGRES_DB=

  # optional, but recommended if you want logs
  RUST_LOG=warn
  ```

- To register the bot, you'll need to register an application at [Discord's Developer Portal](https://discord.dev). The Token should be filled in the `.env` file under the `DISCORD_TOKEN` key.

## Launching

1. Fill the `.env` and `config.json` files with the necessary information.

### Easiest Method
- Review and edit the `docker-compose.yml` file as needed.
- Launch with the following command:
  ```bash
  docker-compose -f docker-compose.yml --env-file .env up
  ```

### Easy Method
- Use either the `launch_bot.sh` or `launch_bot.ps1` scripts.

### Hard Method
- Install `Postgres 13`.
- Install the Rust Toolchain.
- Run the `faculty_manager.sql` file to initialize the database.
- Use your favorite process manager to keep the bot running. Notable mentions include:
  - [pm2](https://pm2.io)
  - systemd
  - openrc



## Bot Settings

To enable the bot to communicate with channels, you need to edit the `config.json` file and add the IDs of the required channels and roles. Ensure these channels and roles are created before launching the bot.

### Roles

- **staffrole**: This is the management role, which can edit the bot via commands.
- **verified**: Assigned to users after they verify their accounts. Use this role to control access to certain channels.
- **mealplannotify**: This role will receive notifications when a new meal plan is posted.

### Channels

- **logs**: Where log files are posted. Useful for staff.
- **greetings**: Where new members are welcomed.
- **news**: Where important information for everyone is posted by the bot.
- **xp**: Where level-up notifications are posted.
- **rules**: Where server rules are located.
- **ads**: Where external members can post ads. Ads are automatically deleted after a specified amount of time set in the settings.
- **createChannel**: When a user joins a channel with this name, a temporary voice channel will be created.
- **mealPlan**: Where meal plan updates are posted.


### Settings

Here you may specify other adjustable settings of the bot.

- **adstimeout**: The time in milliseconds before an ad in the ads channel gets deleted.
- **CharsForLevel**: The number of characters in a message that equal 1 XP.
- **postMealplan**: *(bool)* Activates the meal plan posting functionality.
  - **mealplan**: *(url)* The URL to download the meal plan, e.g., `http://www.meal/one.pdf`.
  - **mealplan-check**: *(u16)* Minutes between meal plan update checks.
  - **postOnDay**: *(String)* The weekday on which the check and post occur ("Monday" - "Sunday").
  - **postAtHour**: *(String)* The hour at which the plan will be posted, e.g., `18:00:00` and `18:30:00` both post at 6 PM for precision's sake.
  - **mealplansettings**: *(list)* Default settings for the converter. Change if applicable.
    - **density**: 400
    - **quality**: 100
    - **flatten**: true
    - **width**: 768
    - **height**: 512

## Commands

- **help**: Displays general information.
- **rulesupdate <"new rules">**: Updates the server rules. Only usable by `staffrole`.
- **sendmessage <channel name> <"message">**: Lets the bot send a message to a channel initially, which can later be updated with the `rulesupdate` command. Only usable by `staffrole`.
- **verify <student email>**: The bot checks the mail inbox and assigns the student the `verified` role.
- **xp**: Displays current XP and level.
- **register**: Registers Discord Slash Commands. Only usable by members with the [MANAGE_GUILD](https://discord.com/developers/docs/topics/permissions#permissions#MANAGE_GUILD) permission.

## Thanks

Feedback is appreciated.
