// This file only allowed to be run on server side
if (typeof window !== 'undefined') {
  throw new Error('This file should only be used on the server side.');
}

const KONG_ADMIN_URL = process.env.DEVLOG_KONG_GATEWAY_ADMIN_URL
const HOST_NAME = process.env.DEVLOG_SERVICE_HOST || 'host.docker.internal'
const PORT = process.env.PORT
const DOMAIN = 'localhost'

let isRegistered = false

export function register() {
  if (!PORT) throw new Error(`This service is only support static port, the env PORT or DEVLOG_SERVICE_PORT must be defined`)

  if (KONG_ADMIN_URL && !isRegistered) {
    isRegistered = true
    registerApiGateway()
  }
}

async function createOrUpdate(endpoint: string, updateEndpoint: string | null, data: any) {
  try {
    const response = await fetch(`${KONG_ADMIN_URL}${endpoint}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    })

    if (response.status === 409) {
      console.warn(`${endpoint} already exists, updating.`)
      const updateResponse = await fetch(`${KONG_ADMIN_URL}${updateEndpoint || endpoint}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      })

      if (!updateResponse.ok) {
        const errorData = await updateResponse.json()
        throw new Error(`Failed to update ${endpoint}: ${JSON.stringify(errorData)}`)
      }

      console.log(`${endpoint} updated successfully.`)
    } 
    else if (!response.ok) {
      const errorData = await response.json()
      throw new Error(`Failed to create ${endpoint}: ${JSON.stringify(errorData)}`)
    }
    else {
      console.log(`${endpoint} created successfully.`)
    }
  }
  catch (error) {
    console.error(`Error handling ${endpoint}:`, error)
  }
}

export async function registerApiGateway() {
  await createOrUpdate('/services', '/services/bitbridge-website', {
    name: 'bitbridge-website',
    url: `http://${HOST_NAME}:${PORT}`,
    path: '/',
  })

  await createOrUpdate('/services/bitbridge-website/routes', '/routes/bitbridge-website-route', {
    expression: `(net.protocol == "http" || net.protocol == "https") && http.path ^= "/" && http.host == "${DOMAIN}"`,
    name: 'bitbridge-website-route',
    priority: 1,
  });
}
