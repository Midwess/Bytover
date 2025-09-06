'use client'

export async function getThumbnailFromFile(
    file: File,
    size: number = 300
): Promise<Uint8Array> {
    const fileType = file.type;

    const isHEIC = fileType === "image/heic" || file.name.toLowerCase().endsWith(".heic");
    const isImage = fileType.startsWith("image/");
    const isVideo = fileType.startsWith("video/");

    if (!isImage && !isVideo && !isHEIC) {
        throw new Error("Unsupported file type for thumbnail.");
    }

    if (isHEIC) {
        try {
            const heic2any = (await import("heic2any")).default;

            const convertedBlob = await heic2any({
                blob: file,
                toType: "image/png",
                quality: 0.2
            }) as Blob;
            
            const convertedFile = new File(
                [convertedBlob],
                file.name.replace(/\.heic$/i, ".png"),
                { type: "image/png" }
            );

            return await getThumbnailFromFile(convertedFile, size);
        } catch (err) {
            throw new Error(`Failed to convert HEIC: ${err}`);
        }
    }

    const url = URL.createObjectURL(file);

    return new Promise<Uint8Array>((resolve, reject) => {
        const cleanup = () => URL.revokeObjectURL(url);

        function drawWithAspect(media: HTMLImageElement | HTMLVideoElement) {
            const naturalWidth = media instanceof HTMLImageElement ? media.naturalWidth : media.videoWidth;
            const naturalHeight = media instanceof HTMLImageElement ? media.naturalHeight : media.videoHeight;
            
            const aspectRatio = naturalWidth / naturalHeight;
            const canvasWidth = size;
            const canvasHeight = Math.round(size / aspectRatio);

            const canvas = document.createElement("canvas");
            canvas.width = canvasWidth;
            canvas.height = canvasHeight;
            const ctx = canvas.getContext("2d");

            if (!ctx) {
                cleanup();
                return reject("Failed to get canvas context");
            }

            ctx.drawImage(media, 0, 0, canvasWidth, canvasHeight);

            canvas.toBlob((blob) => {
                cleanup();
                if (blob) {
                    blob.arrayBuffer().then(buffer => {
                        resolve(new Uint8Array(buffer));
                    }).catch(reject);
                } else {
                    reject("Failed to create thumbnail blob");
                }
            }, "image/png", 0.8);
        }

        if (isImage) {
            const img = new Image();
            img.crossOrigin = "anonymous";
            img.src = url;

            img.onload = () => drawWithAspect(img);
            img.onerror = (e) => {
                cleanup();
                reject(`Failed to load image: ${e}`);
            };
        } else if (isVideo) {
            const video = document.createElement("video");
            video.crossOrigin = "anonymous";
            video.src = url;
            video.preload = "metadata";
            video.muted = true;
            video.playsInline = true;
            video.style.position = "absolute";
            video.style.visibility = "hidden";
            video.style.pointerEvents = "none";
            document.body.appendChild(video);

            video.onloadedmetadata = () => {
                const seekTime = Math.min(video.duration / 2, 4);
                video.currentTime = seekTime;
            };

            video.onseeked = () => {
                drawWithAspect(video);
                video.remove();
            };

            video.onerror = (e) => {
                video.remove();
                cleanup();
                reject(`Failed to load video: ${e}`);
            };
        }
    });
}
