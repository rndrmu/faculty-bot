# Faculty bot (Discord)

## Introduction

This project features a Discord bot, with the intent to reduce the administration overhead for faculty related tasks. The bot takes care of verifying new server members.

## Setting up the bot

- To run the code itself, first you need [The Rust Toolchain](https://rust-lang.org). Install this.
- You need to download the project and execute  
  `cargo build --release` inside the primary folder using Powershell (on Windows). This will compile a release optimized build of the bot


- Additionally, you'll have to create a `.env` file with the following content:
- 
```sh
DISCORD_TOKEN=
MAILUSER=
MAILPW=
SMTP_SERVER=
SMTP_PORT=

RUST_LOG=warn
```

- To register the bot, a developer account at [Discord](https://discord.com/developers/) needs to be created. The key can be filled in the `.env` under `TOKEN`.
- To finally launch the bot, use `./target/{release|debug}/faculty_manager`, depending on if you compiled with the release flag or not.

## Bot Settings

In order for the bot to communicate with channels, you need to edit the `general-settings.json` and paste in the names of the channels and roles. Those need to be created before launching the bot. Case sensitive!

### Roles

- staffrole: this is the management role, which may edit the bot via commands.  
- verified: after a playeer verified with his account. Use this role as you please, maybe to show and hide some channels.
- mealplannotify: role id which will get pinged if new mealplan has been posted

### Channels

- logs : where log files are beeing posted to. Useful for staff.
- greetings : where new players get welcomed
- news : where useful information for everyone gets posted by the bot
- xp : where level ups get posted
- rules : where your server rules are located
- ads: where external members may post ads which automatically get deleted after a specified amout of time in the settings
- createChannel: when creating a voice channel with this name, it will be a dynamic voice channel to prevent unnecessary voice channels
- mealPlan: channel to which the mealplan updates get posted

### Colors

Speficy the color codes which gets used by the bot when sending embedded mssages etc.

### Settings

Here you may speficy other adjustable settings of the bot.
- adstimeout: the time in milliseconds before an ad in the ads-channel gets deleted
- CharsForLevel: how many characters in a message should equal to 1 Point of Experience
- postMealplan: (bool) activates the mealplan posting functionality
	- mealplan : (url) place to download mealplan i.e. http://www.meal/one.pdf
    - mealplanpdfpath": (path) local path to save and load mealplan
    - mealplan-check": (int) minutes between the mealplan update check
    - mealplandaycheck": (int) weekday on which the check and post occurs (0 - 6) (Sun - Sat)
	- mealplanhourscheck: (int) time at what the check occurs. i.e. 8 for 8am
    - mealplansettings": (list) default settings for the converter. change if applicable
      - density": 400,
      - quality": 100,
      - saveFilename": "mensaplan",
      - savePath": "./",
      - format": "png",
      - width": 768,
      - height": 512
    }


## Commands

In general, a command can be used without any arguments to get additional information about it.

- help: displays general information
- rulesupdate <"new rules">: updates the server rules. Only usable by staffrole.
- sendmessage <channel name> <"message">: useful to let the bot send the a the server rules to a channel intitally, which can later be updated with rulesupdate-command. only staffrole.
- verify <student email>: The bot checks the mail inbox and assigns the student the verificated role
- xp: displays current xp and level

## Thanks

Feedback is appreciated.
