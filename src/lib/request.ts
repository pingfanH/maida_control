import axios from 'axios';

const api = axios.create({
    baseURL: 'http://192.168.0.16:9855', // API 基础地址
    headers: {
        'X-User-Id': localStorage.getItem('user_id'),
        'X-Open-Game-Id': localStorage.getItem('open_game_id'),
        'X-Session-Id': localStorage.getItem('open_session_id')
    }
});

export default api;