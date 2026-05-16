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

// Index operations
export async function listIndexes() {
  const { data } = await api.get('/indexes')
  if (Array.isArray(data)) return data
  if (Array.isArray(data?.indexes)) return data.indexes
  return []
}

export async function createIndex(name, schema) {
  const { data } = await api.post(`/indexes/${name}`, { schema })
  return data
}

export async function deleteIndex(name) {
  await api.delete(`/indexes/${name}`)
}

export async function getIndexInfo(name) {
  const { data } = await api.get(`/indexes/${name}`)
  return data
}

// Stats & maintenance
export async function getIndexStats(name) {
  const { data } = await api.get(`/indexes/${name}/stats`)
  return data
}

export async function rebuildIndex(name) {
  const { data } = await api.post(`/indexes/${name}/rebuild`)
  return data
}

export async function compressIndex(name) {
  const { data } = await api.post(`/indexes/${name}/compress`)
  return data
}

// Document operations
export async function addDoc(name, doc) {
  const { data } = await api.post(`/indexes/${name}/docs`, doc)
  return data
}

export async function bulkAddDocs(name, docs) {
  const { data } = await api.post(`/indexes/${name}/docs/_bulk`, docs)
  return data
}

export async function listDocs(name, limit = 10, offset = 0) {
  const { data } = await api.get(`/indexes/${name}/docs`, { params: { limit, offset } })
  return data
}

export async function getDoc(name, field, value) {
  const { data } = await api.get(`/indexes/${name}/docs/${field}/${encodeURIComponent(value)}`)
  return data
}

export async function deleteDoc(name, field, value) {
  const { data } = await api.delete(`/indexes/${name}/docs/${field}/${encodeURIComponent(value)}`)
  return data
}

// Search
export async function searchIndex(name, q, limit = 10, offset = 0, highlight = []) {
  const { data } = await api.get(`/indexes/${name}/search`, {
    params: { q, limit, offset, highlight }
  })
  return data
}

export async function searchIndexPost(name, body) {
  const { data } = await api.post(`/indexes/${name}/search`, body)
  return data
}
