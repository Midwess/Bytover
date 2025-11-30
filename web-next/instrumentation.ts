// This file only allowed to be run on server side
import { promises as fs } from 'fs';
import path from 'path';
import { S3Client, PutObjectCommand, HeadObjectCommand } from '@aws-sdk/client-s3';
import mime from 'mime-types';
import Bluebird from 'bluebird';

if (typeof window !== 'undefined') {
  throw new Error('This file should only be used on the server side.');
}

const KONG_ADMIN_URL = process.env.KONG_GATEWAY_ADMIN_URL
const HOST_NAME = process.env.SERVICE_HOST || 'host.docker.internal'
const PORT = process.env.PORT
const DOMAIN = 'localhost'

const S3_CDN_PREFIX = process.env.S3_CDN_PREFIX || ''
// The commit hash
const VERSION = process.env.VERSION || process.env.RAILWAY_GIT_COMMIT_SHA

let isRegistered = false

export function register() {
  if (!PORT) throw new Error(`This service is only support static port, the env PORT or SERVICE_PORT must be defined`)

  if (KONG_ADMIN_URL && !isRegistered) {
    isRegistered = true
    registerApiGateway()
  }

  if (VERSION && S3_CDN_PREFIX) {
    setupCDN()
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
    expression: `(net.protocol == "http" || net.protocol == "https") && http.path ^= "/" && http.host == "${DOMAIN}" && http.method != "POST"`,
    name: 'bitbridge-website-route',
    priority: 0,  // Lowest priority (fallback)
  });
}

export async function setupCDN(): Promise<void> {
  // Validate configuration
  if (!VERSION || !S3_CDN_PREFIX || S3_CDN_PREFIX === '/') {
    console.warn('Invalid configuration: VERSION or S3_CDN_PREFIX is missing.');
    return;
  }

  try {
    const entry = `${__dirname}/../../`;
    const ns = 'cdn-uploader';
    console.log(ns, 'Entry point:', entry);

    // Initialize S3 client
    const s3 = new S3Client({ region: process.env.AWS_REGION || 'us-east-1' });

    const publicDir = path.resolve(entry, 'public');
    const nextStaticDir = path.resolve(entry, '.next/static');
    const bucketBase = `midwess/bytover/web/commit-${VERSION}`;
    const acl = 'public-read';

    // Function to check if a file exists in the bucket
    const fileExistsInS3 = async (bucket: string, key: string): Promise<boolean> => {
      try {
        await s3.send(
          new HeadObjectCommand({
            Bucket: bucket,
            Key: key,
          })
        );
        return true; // File exists
      } catch (error: any) {
        if (error.name === 'NotFound') {
          return false; // File does not exist
        }
        throw error; // Other errors should propagate
      }
    };

    // Function to upload files recursively
    const uploadDirectory = async (dirPath: string, s3Path: string): Promise<void> => {
      const entries = await fs.readdir(dirPath, { withFileTypes: true });

      await Bluebird.map(entries, async (entry: any) => {
        const fullPath = path.join(dirPath, entry.name);
        const s3Key = `${s3Path}/${entry.name}`;

        if (entry.isDirectory()) {
          // Recursively upload subdirectories
          await uploadDirectory(fullPath, s3Key);
        } else {
          // Check if the file already exists
          const fileAlreadyExists = await fileExistsInS3(process.env.AWS_S3_BUCKET_NAME!, s3Key);

          if (fileAlreadyExists) {
            return;
          }

          // Determine the Content-Type
          const contentType = mime.lookup(fullPath) || 'application/octet-stream';

          // Read file and upload
          const fileContent = await fs.readFile(fullPath);

          const command = new PutObjectCommand({
            Bucket: process.env.AWS_S3_BUCKET_NAME!,
            Key: s3Key,
            Body: fileContent,
            ACL: acl,
            ContentType: contentType, // Set the Content-Type
          });

          await s3.send(command);
          console.log(ns, `Uploaded: ${s3Key} with Content-Type: ${contentType}`);
        }
      }, { concurrency: 30 });
    };

    // Upload public directory
    await uploadDirectory(publicDir, `${bucketBase}`);

    // Upload .next/static directory
    await uploadDirectory(nextStaticDir, `${bucketBase}/_next/static`);

    console.log(ns, 'Upload completed successfully.');
  } catch (error) {
    console.error('Error uploading to CDN:', error);
    throw error;
  }
}