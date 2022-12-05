import {
  Client,
  Collection,
  Message,
  MessageAttachment,
  TextChannel,
} from "discord.js";
import {
  logMessage,
  toLevel,
  applyText,
  toHHMMSS,
} from "../functions/extensions";
import Canvas from "canvas";
import Keyv from "keyv";
import settings from "../../general-settings.json";
import config from "../../config.json";

import { LooseObject } from "../index";

module.exports = {
  event: "messageCreate",
  async execute(client: LooseObject, [message]: [Message]) {
    if (message.author.bot) return; // bye bye robots
    console.log(message.content);
    const regex = new RegExp(
      `^(<@!?${client.user?.id}>|${config.prefix.toLowerCase()})\\s*\\w*`
    );
    // prefix should also be mention

    // no command! - simple message to track for XP
    if (
      message.content.toLocaleLowerCase().startsWith(`verify`) ||
      message.content.toLocaleLowerCase().startsWith(`.. verify`)
    ) {
      message
        .reply("You need to use ..verify")
        .then(() => message.delete())
        .catch(console.error);
    }

    if (!message.content.toLocaleLowerCase().startsWith(`..`)) {
      // handle ads
      /*  if ((message.channel as TextChannel ).id == settings.channels.ads) {
       // TODO extend to database + calculation for the case the bot crashes
       var deletionDate = new Date();
       deletionDate.setMilliseconds(
         (deletionDate.getMilliseconds() as number) + settings.settings.adstimeout
       );
       console.info(`ad posted. Will be deleted on ${deletionDate}`);
       // put into db
       const adsdb = new Keyv("sqlite://ads.sqlite");
        adsdb.on("error", (err: any) => console.error("Keyv connection error:", err));
       adsdb.set(message.id, deletionDate);
       //message.delete(); // 4 weeks
     } */

      // Key: iD, Value: XP
      const dbxp = new Keyv("sqlite://xp.sqlite");
      dbxp.on("error", (err: any) =>
        console.error("Keyv connection error:", err)
      );

      const userXP = await dbxp.get(message.author.id);

      // user xp
      if (!userXP || userXP === undefined) {
        await dbxp.set(message.author.id, 1); // set to 1 for 1 XP
        return;
      } else {
        // if new level, post XP
        let newXP =
          userXP +
          message.content.length /
          parseFloat(settings.settings.CharsForLevel.toString());

        // check if newXP is +100 of prev XP
        if (newXP >= userXP + 100) {
          // send level xp to xp channel
          const canvas = Canvas.createCanvas(700, 250);
          const ctx = canvas.getContext("2d");

          // Since the image takes time to load, you should await it
          const background = await Canvas.loadImage("../images/banner.png");
          // This uses the canvas dimensions to stretch the image onto the entire canvas
          ctx.drawImage(background, 0, 0, canvas.width, canvas.height);
          // Select the color of the stroke
          ctx.strokeStyle = "#74037b";
          // Draw a rectangle with the dimensions of the entire canvas
          ctx.strokeRect(0, 0, canvas.width, canvas.height);

          // Select the font size and type from one of the natively available fonts
          ctx.font = "60px sans-serif";

          // Slightly smaller text placed above the member's display name
          ctx.font = applyText(
            canvas,
            `${message.author.username} has reached`
          );
          ctx.fillStyle = "#ffffff";
          ctx.fillText(
            `${message.author.username} has reached`,
            canvas.width / 2.4,
            canvas.height / 3.5
          );

          // Add an exclamation point here and below
          ctx.font = applyText(
            canvas,
            `LEVEL ${Math.trunc(toLevel(userXP) + 1)}`
          );
          ctx.fillStyle = "#ffffff";
          ctx.fillText(
            `LEVEL ${Math.trunc(toLevel(userXP) + 1)}`,
            canvas.width / 2.4,
            canvas.height / 1.5
          );

          // Use helpful Attachment class structure to process the file for you
          const attachment = new MessageAttachment(
            canvas.toBuffer(),
            "level-up-image.png"
          );
          console.log();
          let lvlmsg = (await message.guild?.channels.cache
            .find((chn) => chn.name == settings.channels.xp)
            ?.fetch()) as TextChannel;

          lvlmsg.send({
            content: `Congrats, <@${message.author}>!`,
            files: [attachment],
          });

          // `Congrats, ${message.author}!`, attachment
        }

        // New 1 XP for 200 chars. That means 200 chars equals to one XP Point
        const lengthPoints = message.content.length / parseFloat(settings.settings.CharsForLevel.toString());

        dbxp.set(
          message.author.id,
          userXP + lengthPoints
        );
        // console.log("XP: " + userXP + " + " + lengthPoints + " = " + (userXP + lengthPoints));
      }

      return;
    }

    //filter args
    const args = message.content.slice(config.prefix.length).split(/ +/); //filter args
    const commandName = args.shift()?.toLowerCase();
    console.log({
      commandName,
      args,
    });

    //command checking aliases
    const command =
      client.commands.get(commandName) ||
      client.commands.find(
        (cmd: any) => cmd.aliases && cmd.aliases.includes(commandName)
      );
        console.log(command);
    if (!command) return;

    //error checking
    //guild check
    if (command.guildOnly && !message.channel.isText()) {
      return message.reply("I can't execute that command inside DMs!");
    }

    //args check
    if (command.args && !args.length) {
      let reply = `You didn't provide any arguments, ${message.author}!`;

      if (command.usage) {
        reply += `\nThe proper usage would be: \`${process.env.PREFIX}${command.name} ${command.usage}\``;
      }
      return message.channel.send(reply);
    }

    // cooldown
    if (!client.cooldowns.has(command.name)) {
      client.cooldowns.set(command.name, new Collection());
    }

    const now = Date.now();
    const timestamps = client.cooldowns.get(command.name);
    const cooldownAmount = (command.cooldown || 1) * 1000;
    if (timestamps.has(message.author.id)) {
      const expirationTime = timestamps.get(message.author.id) + cooldownAmount;

      if (now < expirationTime) {
        const timeLeft = (expirationTime - now) / 1000;
        return message.reply(
          `please wait ${toHHMMSS(timeLeft.toFixed(1))} before reusing the \`${command.name
          }\` command.`
        );
      }
    } else {
      timestamps.set(message.author.id, now);
      setTimeout(() => timestamps.delete(message.author.id), cooldownAmount);
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Execute command
    // ----------------------------------------------------------------------------------------------------------------------------------------------
    try {
      command.execute(message, args);
    } catch (error) {
      console.error(error);
      message.reply("there was an error trying to execute that command!");
    }
  },
};
