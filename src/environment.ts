import { config } from 'dotenv';

config();

export const DISCORD_TOKEN = process.env.DISCORD_TOKEN;
export const CLIENT_ID = process.env.CLIENT_ID;
export const HOME_GUILD_ID = process.env.HOME_GUILD_ID;
export const RTP_ROLE_ID = process.env.RTP_ROLE_ID;
