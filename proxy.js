// Node.js 示例
import express from "express";
import fetch from "node-fetch"
const app = express();

app.get('/aime', async (req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET,POST,OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

    const r = await fetch('https://tgk-wcaime.wahlap.com/wc_auth/oauth/authorize/maimai-dx', { redirect: 'manual' });
    const location = r.headers.get('location');
    res.json({ location });
});

app.listen(3000);
