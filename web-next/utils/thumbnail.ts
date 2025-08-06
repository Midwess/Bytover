'use client'

export async function getThumbnailFromFile(
    file: File,
    size: number = 300
): Promise<Blob> {
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

    return new Promise<Blob>((resolve, reject) => {
        const cleanup = () => URL.revokeObjectURL(url);

        function drawWithAspect(media: HTMLImageElement | HTMLVideoElement) {
            let { width: w, height: h } = media.getBoundingClientRect();
            w = w || media.width;
            h = h || media.height;

            const canvas = document.createElement("canvas");
            canvas.width = w;
            canvas.height = h;
            const ctx = canvas.getContext("2d");

            if (!ctx) {
                cleanup();
                return reject("Failed to get canvas context");
            }

            ctx?.drawImage(media, 0, 0, w, h);

            canvas.toBlob((blob) => {
                cleanup();
                blob ? resolve(blob) : reject("Failed to create thumbnail blob");
            }, "image/png");
        }

        if (isImage) {
            const img = new Image();
            img.crossOrigin = "anonymous";
            img.src = url;
            img.width = size;
            img.style.aspectRatio = '';
            img.style.display = 'block';

            img.onload = () => {
                const aspectRatio = img.naturalWidth / img.naturalHeight;
                img.height = Math.round(250 / aspectRatio);

                drawWithAspect(img);
            }
            img.onerror = (e) => {
                cleanup();
                reject(`Failed to load image: ${e}`);
            };
        } else if (isVideo) {
            const video = document.createElement("video");
            video.crossOrigin = "anonymous";
            video.src = url;
            video.preload = "auto";
            video.muted = true;
            video.playsInline = true;
            video.style.position = "absolute";
            video.style.width = `${size}px`;
            video.style.height = 'auto';
            video.style.display = 'block';
            video.style.aspectRatio = '';
            video.style.pointerEvents = "none";
            video.style.visibility = "hidden";
            document.body.appendChild(video);

            video.onloadeddata = () => {
                const seekTime = Math.min(Math.min(video.duration / 2, 4), Math.max(0, video.duration - 0.1));
                video.currentTime = seekTime;
            };

            video.onseeked = () => {
                drawWithAspect(video)
                video.remove(); // Clean up even on error
            };

            video.onerror = (e) => {
                video.remove(); // Clean up even on error
                cleanup();
                reject(`Failed to load video: ${e}`);
            };
        }
    });
}
