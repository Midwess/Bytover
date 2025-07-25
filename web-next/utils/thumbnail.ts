export async function getThumbnailFromFile(
    file: File,
    size: number = 128
): Promise<Blob> {
    const fileType = file.type;

    const isImage = fileType.startsWith("image/");
    const isVideo = fileType.startsWith("video/");

    if (!isImage && !isVideo) {
        throw new Error("Unsupported file type for thumbnail.");
    }

    const url = URL.createObjectURL(file);

    return new Promise<Blob>((resolve, reject) => {
        const cleanup = () => URL.revokeObjectURL(url);

        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d");

        if (!ctx) {
            cleanup();
            return reject("Failed to get canvas context");
        }

        if (isImage) {
            const img = new Image();
            img.crossOrigin = "anonymous";
            img.src = url;

            img.onload = () => {
                ctx.drawImage(img, 0, 0, size, size);
                canvas.toBlob((blob) => {
                    cleanup();
                    blob ? resolve(blob) : reject("Failed to create thumbnail blob");
                }, "image/png");
            };

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

            video.onloadeddata = () => {
                // Seek to 1s (or max 0.1s before end)
                const seekTime = Math.min(1, Math.max(0, video.duration - 0.1));
                video.currentTime = seekTime;
            };

            video.onseeked = () => {
                ctx.drawImage(video, 0, 0, size, size);
                canvas.toBlob((blob) => {
                    cleanup();
                    blob ? resolve(blob) : reject("Failed to create thumbnail blob");
                }, "image/png");
            };

            video.onerror = (e) => {
                cleanup();
                reject(`Failed to load video: ${e}`);
            };
        }
    });
}
