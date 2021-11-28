use crate::ShardManagerContainer;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::prelude::*,
    prelude::*,
    
};


#[group]
#[commands(xp, leaderboard)]
#[description = "Levels commands."]
struct Levels;

#[command]
#[description = "Shows your current level and XP."]
#[usage = ""]
#[example = ""]
#[aliases("level")]
#[bucket = "level"]
async fn xp(ctx: &Context, msg: &Message) -> CommandResult {
    
    Ok(())
}

#[command]
#[description = "Shows the leaderboard."]
async fn leaderboard(ctx: &Context, msg: &Message) -> CommandResult {
    
    Ok(())
}