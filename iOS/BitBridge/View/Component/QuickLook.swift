//
//  QuickLook.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 31/5/25.
//

import SwiftUI
import Foundation
import QuickLook
import SharedTypes
import Photos
import PhotosUI
import AVKit
import WebKit

struct QuickLook: View {
    @Binding var path: LocalResourcePath?
    @State private var isLoading = true

    var body: some View {
        Group {
            if let path = path {
                ZStack {
                    switch path {
                    case .platformIdentifier(let identifier) where identifier.hasPrefix("phasset://"):
                        let assetId = String(identifier.dropFirst("phasset://".count))
                        PhassetViewer(assetIdentifier: assetId)
                            .onAppear {
                                isLoading = false
                            }
                    default:
                        FileViewer(path: path)
                            .onAppear {
                                isLoading = false
                            }
                    }

                    if isLoading {
                        ProgressView()
                            .progressViewStyle(CircularProgressViewStyle())
                            .scaleEffect(1.5)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                            .background(Color.black.opacity(0.3))
                    }
                }
                .onDisappear {
                    self.path = nil
                    isLoading = true
                }
            }
        }
    }
}

struct QuickLookSheet: View {
    @Binding var path: LocalResourcePath?
    @State private var isPresented = false

    var body: some View {
        Text("")
            .onChange(of: path) { newPath in
                isPresented = newPath != nil
            }
            .sheet(isPresented: $isPresented, onDismiss: {
                path = nil
            }) {
                if let path = path {
                    NavigationView {
                        QuickLook(path: .constant(path))
                            .navigationBarTitleDisplayMode(.inline)
                            .navigationBarItems(
                                leading: Button("Open") {
                                    if let fileURL = getFileURL(for: path) {
                                        print("Openning \(fileURL)")
                                        UIApplication.shared.open(fileURL, options: [.universalLinksOnly: true], completionHandler: {result in
                                            print("result open \(result)")
                                        })
                                    }
                                },
                                trailing: Button("Done") {
                                    isPresented = false
                                }
                            )
                    }
                    .presentationDetents([.large])
                    .presentationDragIndicator(.visible)
                }
            }
    }

    private func getFileURL(for path: LocalResourcePath) -> URL? {
        switch path {
        case .absolutePath(let absolutePath):
            return URL(fileURLWithPath: absolutePath)

        case .platformIdentifier(let identifier):
            if identifier.hasPrefix("bookmark://") {
                let bookmarkString = String(identifier.dropFirst("bookmark://".count))
                if let bookmarkData = Data(base64Encoded: bookmarkString) {
                    var isStale = false
                    do {
                        guard let url = try? URL(resolvingBookmarkData: bookmarkData, relativeTo: nil, bookmarkDataIsStale: &isStale) else {
                            return nil
                        }

                        if url.startAccessingSecurityScopedResource() {
                            return url
                        }
                    } catch {}
                }
            }
            return nil

        case .relativePath(let relativePath, let isPrivate):
            let baseURL = getDocumentsDirectory(isPrivate: isPrivate)
            return baseURL.appendingPathComponent(relativePath)
        }
    }

    private func getDocumentsDirectory(isPrivate: Bool) -> URL {
        let privatePath = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        let publicPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first

        return isPrivate ? privatePath! : publicPath!
    }
}

struct PhassetViewer: View {
    let assetIdentifier: String
    @State private var asset: PHAsset?
    @State private var image: UIImage?
    @State private var livePhoto: PHLivePhoto?
    @State private var player: AVPlayer?
    @State private var isLoading = true

    var body: some View {
        ZStack {
            Color.black
                .ignoresSafeArea()

            if isLoading {
                ProgressView()
                    .progressViewStyle(CircularProgressViewStyle())
                    .scaleEffect(1.5)
                    .foregroundColor(.white)
            } else if let asset = asset {
                switch asset.mediaType {
                case .image:
                    if asset.mediaSubtypes.contains(.photoLive), let livePhoto = livePhoto {
                        LivePhotoView(livePhoto: livePhoto)
                    } else if let image = image {
                        Image(uiImage: image)
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                    }
                case .video:
                    if let player = player {
                        VideoPlayer(player: player)
                            .onAppear {
                                player.play()
                            }
                    }
                default:
                    Text("Unsupported file format")
                }
            }
        }
        .onAppear {
            loadAsset()
        }
    }

    private func loadAsset() {
        Task {
            guard let assetCached = await PHAsset.getCachedAsset(identifier: assetIdentifier, false) else {
                await MainActor.run {
                    isLoading = false
                }
                return
            }

            let asset = assetCached.asset

            await MainActor.run {
                self.asset = asset
            }

            switch asset.mediaType {
            case .image:
                if asset.mediaSubtypes.contains(.photoLive) {
                    await loadLivePhoto(for: asset)
                } else {
                    await loadImage(for: asset)
                }
            case .video:
                await loadVideo(for: asset)
            default:
                await loadImage(for: asset)
            }

            await MainActor.run {
                isLoading = false
            }
        }
    }

    private func loadImage(for asset: PHAsset) async {
        let options = PHImageRequestOptions()
        options.isNetworkAccessAllowed = true
        options.deliveryMode = .highQualityFormat

        await withCheckedContinuation { continuation in
            PHImageManager.default().requestImage(for: asset, targetSize: PHImageManagerMaximumSize, contentMode: .aspectFit, options: options) { image, _ in
                Task { @MainActor in
                    self.image = image
                    continuation.resume()
                }
            }
        }
    }

    private func loadLivePhoto(for asset: PHAsset) async {
        let options = PHLivePhotoRequestOptions()
        options.isNetworkAccessAllowed = true
        options.deliveryMode = .highQualityFormat

        await withCheckedContinuation { continuation in
            PHImageManager.default().requestLivePhoto(for: asset, targetSize: PHImageManagerMaximumSize, contentMode: .aspectFit, options: options) { livePhoto, _ in
                Task { @MainActor in
                    self.livePhoto = livePhoto
                    continuation.resume()
                }
            }
        }
    }

    private func loadVideo(for asset: PHAsset) async {
        let options = PHVideoRequestOptions()
        options.isNetworkAccessAllowed = true
        options.deliveryMode = .highQualityFormat

        await withCheckedContinuation { continuation in
            PHImageManager.default().requestAVAsset(forVideo: asset, options: options) { avAsset, _, _ in
                Task { @MainActor in
                    if let urlAsset = avAsset as? AVURLAsset {
                        self.player = AVPlayer(url: urlAsset.url)
                    }
                    continuation.resume()
                }
            }
        }
    }
}

struct LivePhotoView: UIViewRepresentable {
    let livePhoto: PHLivePhoto

    func makeUIView(context: Context) -> PHLivePhotoView {
        let livePhotoView = PHLivePhotoView()
        livePhotoView.contentMode = .scaleAspectFit
        livePhotoView.livePhoto = livePhoto
        return livePhotoView
    }

    func updateUIView(_ uiView: PHLivePhotoView, context: Context) {
        uiView.livePhoto = livePhoto
    }
}

struct FileViewer: View {
    let path: LocalResourcePath
    @State private var isLoading = true

    var body: some View {
        ZStack {
            Group {
                if let fileURL = getFileURL() {
                    QuickLookPreview(fileURL: fileURL)
                        .onAppear {
                            isLoading = false
                        }
                } else {
                    Text("Unable to load file")
                        .foregroundColor(.red)
                        .onAppear {
                            isLoading = false
                        }
                }
            }

            if isLoading {
                ProgressView()
                    .progressViewStyle(CircularProgressViewStyle())
                    .scaleEffect(1.5)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(Color.black.opacity(0.3))
            }
        }
    }

    private func getFileURL() -> URL? {
        switch path {
        case .absolutePath(let absolutePath):
            return URL(fileURLWithPath: absolutePath)

        case .platformIdentifier(let identifier):
            if identifier.hasPrefix("bookmark://") {
                let bookmarkString = String(identifier.dropFirst("bookmark://".count))
                if let bookmarkData = Data(base64Encoded: bookmarkString) {
                    var isStale = false
                    do {
                        guard let url = try? URL(resolvingBookmarkData: bookmarkData, relativeTo: nil, bookmarkDataIsStale: &isStale) else {
                            return nil
                        }

                        if url.startAccessingSecurityScopedResource() {
                            return url
                        }
                    } catch {}
                }
            }
            return nil

        case .relativePath(let relativePath, let isPrivate):
            let baseURL = getDocumentsDirectory(isPrivate: isPrivate)
            return baseURL.appendingPathComponent(relativePath)
        }
    }

    private func getDocumentsDirectory(isPrivate: Bool) -> URL {
        let privatePath = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        let publicPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first

        return isPrivate ? privatePath! : publicPath!
    }
}

struct QuickLookPreview: UIViewControllerRepresentable {
    let fileURL: URL

    func makeCoordinator() -> Coordinator {
        Coordinator(fileURL: fileURL)
    }

    func makeUIViewController(context: Context) -> QLPreviewController {
        let controller = QLPreviewController()
        controller.dataSource = context.coordinator
        return controller
    }

    func updateUIViewController(_ uiViewController: QLPreviewController, context: Context) {}

    class Coordinator: NSObject, QLPreviewControllerDataSource {
        let fileURL: URL

        init(fileURL: URL) {
            self.fileURL = fileURL
        }

        func numberOfPreviewItems(in controller: QLPreviewController) -> Int {
            1
        }

        func previewController(_ controller: QLPreviewController, previewItemAt index: Int) -> QLPreviewItem {
            return fileURL as QLPreviewItem
        }
    }
}

class PHAssetPreviewItem: NSObject, QLPreviewItem {
    let assetIdentifier: String
    private var _previewItemURL: URL?
    private var _previewItemTitle: String?

    init(assetIdentifier: String) {
        self.assetIdentifier = assetIdentifier
        super.init()
        loadAssetData()
    }

    var previewItemURL: URL? {
        return _previewItemURL
    }

    var previewItemTitle: String? {
        return _previewItemTitle
    }

    private func loadAssetData() {
        Task {
            if let assetCached = await PHAsset.getCachedAsset(identifier: assetIdentifier) {
                await MainActor.run {
                    self._previewItemURL = assetCached.fileUrl
                    self._previewItemTitle = assetCached.fileName
                }
            }
        }
    }
}

#Preview {
    VStack {
        // Example with absolute path
        QuickLookSheet(path: .constant(.absolutePath("/Users/tiendang/Downloads/IMG_4296.png")))

//        // Example with relative path (private)
//        QuickLook(path: .constant(.relativePath(path: "example.pdf", is_private: true)))
//        
//        // Example with platform identifier (PHAsset)
//        QuickLook(path: .constant(.platformIdentifier("phasset://12345-ABCDE-67890")))
//        
//        // Example with platform identifier (bookmark)
//        QuickLook(path: .constant(.platformIdentifier("bookmark://SGVsbG8gV29ybGQ=")))
    }
}
