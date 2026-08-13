import axios from 'axios';

export const apiClient = axios.create({
  baseURL: '/api/plugins/files',
});

// Response interceptor for error handling
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    // Let the calling code handle the error
    return Promise.reject(error);
  }
);
