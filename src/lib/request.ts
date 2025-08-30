import axios from 'axios';

const api = axios.create({
    baseURL: 'http://192.168.10.9:9855', // API 基础地址
    headers: {
        'X-User-Id': localStorage.getItem('user_id'),
        'X-Session-Id': localStorage.getItem('session_id'),
        'X-Open-User-Id': localStorage.getItem('open_user_id')
    }
});

const dxNet = (method)=>{
    return api.get("/api",{
        headers:{
            "method":method
        }
    })
};
export {api, dxNet};