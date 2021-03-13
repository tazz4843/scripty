# coding=utf-8
"""
The centralized handler for Scripty. It handles connections to bot-wide servers such as Statcord or the TTS API.

Client Opcodes:
    0: IDENTIFY
        this MUST be the first payload sent, otherwise one will get disconnected with invalid auth
        contains one key:
            "auth": the authentication key specified in config.json.
        if this succeeded, the server will respond with 0 Authorized and the client is free to begin sending packets.

    1: SERVER_COUNT
        should be sent on average, hourly
        contains two keys,
            "count": the number of servers handled by this cluster
            "cluster": the cluster ID that was passed to it

    2: USER_COUNT
        should be sent on average, hourly as well.
        contains two keys:
            "count": the number of users that this shard can see
            "cluster": the cluster ID that was passed to it

    3: REGISTER_VOICE_CHANNELS
        optional, but allows the server to more efficiently process data, and allows for "cluster" to be left out of
        CALL_TTS_API payloads.
        should be sent directly after receiving code 0 if ever, and if it was sent,
        repeatedly sent once every 15 minutes.
        contains two keys:
            "cluster": the cluster this originates from.
            "vcs": a list of voice chat IDs that this cluster handles.

    4: CALL_TTS_API
        contains 4 keys:
            "data": the raw PCM data. there does not need to be a header, or for it to be more than one second long.
            all of that will be checked by the server.
            "vc_id": the voice chat ID that this data came from.
            "nonce": a nonce to respond with
            "cluster": the cluster this was sent from.
        will eventually invoke a code 4 response, perhaps without a payload because it timed out after 24 hours.

    5: FETCH_USER
        fetch a user from the DB.
        contains 3 keys:
            "user_id": the user ID to fetch
            "nonce": a nonce that should be returned
            "cluster": the cluster this was sent from
        this will invoke a code 5 response

    6: FETCH_GUILD
        fetch a guild from the DB.
        contains 3 keys:
            "guild_id": the guild ID to fetch
            "nonce": a nonce that should be returned
            "cluster": the cluster this was sent from
        this will invoke a code 6 response

    7: FETCH_CHANNEL
        fetch a channel from the DB.
        contains 4 keys:
            "channel_id": int: the channel ID to fetch
            "voice": bool: is this a voice channel? will change the response type from 7 to 8 if true
            "nonce": int: a nonce that should be returned
            "cluster": int: the cluster this was sent from
        this will invoke a code 7 or 8 response, depending on "voice".

    8: UNUSED
        reserved as code 8 response is used for voice

Server Opcodes:
    0: AUTHORIZED
        sent if the token in a code 0 IDENTIFY is valid, this means the client is free to send packets

    1: UNUSED
        reserved as code 1 message does not return response

    2: UNUSED
        reserved as code 2 message does not return response

    3: UNUSED
        reserved as code 3 message does not return response

    4: TTS_API_RESPONSE
        sent upon getting a response from the TTS API about a code 4.
        contains 4 fields:
            "transcript": the actual transcript. this is what should be used most of the time
            "raw_data": raw JSON response
            "nonce": the nonce that was sent with the original code 4
            "vc_id": the VC this was sent from, as specified in code 4

    5: USER_RESPONSE
        sent as response to code 5 FETCH_USER.

"""
import websockets
import asyncio
import json


async def echo(ws: websockets.WebSocketServerProtocol, path: str):
    async for message in ws:
        try:
            data = json.loads(message)
        except json.JSONDecodeError:
            await ws.send("{\"error\": \"invalid JSON\"}")
            continue
        if code := data.get("code"):
            if code == "1":
                pass
        else:
            await ws.send("{\"error\": \"no code passed in JSON\"}")
