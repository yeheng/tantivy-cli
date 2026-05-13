import axios from 'axios'

const api = axios.create({
  baseURL: '',
  headers: { 'Content-Type': 'application/json' },
})

api.interceptors.response.use(
  (r) => r,
  (err) => {
    const msg = err.response?.data?.error || err.message
    return Promise.reject(new Error(msg))
  }
)

export default api

export async function listIndexes() {
  const { data } = await api.get('/indexes')
  return data
}

export async function createIndex(name, schema) {
  await api.post(`/indexes/${name}`, { schema })
}

export async function deleteIndex(name) {
  await api.delete(`/indexes/${name}`)
}

export async function getIndexStats(name) {
  const { data } = await api.get(`/indexes/${name}/stats`)
  return data
}

export async function searchIndex(name, q, limit = 10) {
  const { data } = await api.get(`/indexes/${name}/search`, { params: { q, limit } })
  return data
}

export async function addDoc(name, doc) {
  const { data } = await api.post(`/indexes/${name}/docs`, doc)
  return data
}

export async function listDocs(name, limit = 10, offset = 0) {
  const { data } = await api.get(`/indexes/${name}/docs`, { params: { limit, offset } })
  return data
}

export async function rebuildIndex(name) {
  await api.post(`/indexes/${name}/rebuild`)
}

export async function compressIndex(name) {
  await api.post(`/indexes/${name}/compress`)
}
