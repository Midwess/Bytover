//
//  CoreMock.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 20/9/25.
//

import SharedTypes

@MainActor
class CoreMock: Core {
    static func empty() -> Core {
        CoreMock() as Core
    }

    static func withSelectedFileTransfers() -> Core {
        let x = CoreMock() as Core

        // Create avatar view models
        let bearAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Bear.png?r=146&g=108&b=85", dominant_color_r: 146, dominant_color_g: 108, dominant_color_b: 85)
        let foxAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Fox.png?r=221&g=155&b=104", dominant_color_r: 221, dominant_color_g: 155, dominant_color_b: 104)
        let wolfAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Wolf.png?r=128&g=128&b=128", dominant_color_r: 128, dominant_color_g: 128, dominant_color_b: 128)

        // Create resource view models
        let path = LocalResourcePath.absolutePath("")
        let resource1 = SelectedResourceViewModel(order_id: 1, name: "ScreenShot.png", size_gb: 0, size_mb: 2.0, display_path: "/Photos/ScreenShot.png", path: path, thumbnail_path: nil, type: .image)
        let resource2 = SelectedResourceViewModel(order_id: 2, name: "Document.pdf", size_gb: 0, size_mb: 5.3, display_path: "/Documents/Document.pdf", path: path, thumbnail_path: nil, type: .file)
        let resource3 = SelectedResourceViewModel(order_id: 3, name: "Video.mp4", size_gb: 0.25, size_mb: 256, display_path: "/Videos/Video.mp4", path: path, thumbnail_path: nil, type: .video)

        // Create receive sessions
        let receive_session1 = ReceiveSessionViewModel(
            id: 1,
            peer_avatar: bearAvatar,
            peer_name: "Tien Dang",
            peer_description: "nearby",
            image_resources: [
                ImageReceiveResourceViewModel(model: resource1, completion: 1.0, is_completed: false)
            ],
            video_resources: [],
            file_resources: [],
            is_completed: false,
            is_in_progress: true,
            display_download_speed: "2.0 MB/s",
            progress: 0.8,
            display_datetime: "2025-08-22 12:44"
        )

        let receive_session2 = ReceiveSessionViewModel(
            id: 2,
            peer_avatar: foxAvatar,
            peer_name: "Alex Smith",
            peer_description: "nearby",
            image_resources: [],
            video_resources: [],
            file_resources: [
                FileReceiveResourceViewModel(model: resource2, completion: 0.8, is_completed: false)
            ],
            is_completed: false,
            is_in_progress: true,
            display_download_speed: "1.5 MB/s",
            progress: 0.45,
            display_datetime: "2025-08-22 12:44"
        )

        let receive_session3 = ReceiveSessionViewModel(
            id: 3,
            peer_avatar: wolfAvatar,
            peer_name: "Sarah Johnson",
            peer_description: "nearby",
            image_resources: [],
            video_resources: [
                VideoReceiveResourceViewModel(model: resource3, completion: 1.0, is_completed: true)
            ],
            file_resources: [],
            is_completed: true,
            is_in_progress: false,
            display_download_speed: "0 KB/s",
            progress: 1.0,
            display_datetime: "2025-08-22 12:44"
        )

        // Initialize the transfer view model
        x.transfer = .init(TransferViewModel(
            transfer_method: .device,
            nearby_peers: [],
            received_sessions: [receive_session1, receive_session2, receive_session3],
            received_cloud_sessions: [],
            cloud_session: CloudSession(access_url: "https://bitbridge.devlog.studio/12384", password: "secure123!", session_id: 12384, is_completed: false, is_in_progress: true, display_download_speed: "1.2 MB/s", progress: 0.88)
        ))

        // Add selected resources
        x.shelf.value?.selected_resources.append(SelectedResourceViewModel(order_id: 10, name: "Screenshot", size_gb: 0.02, size_mb: 20, display_path: "xyz", path: path, thumbnail_path: nil, type: .image))
        x.shelf.value?.selected_resources.append(SelectedResourceViewModel(order_id: 11, name: "Folder 102384921", size_gb: 1.2, size_mb: 1200, display_path: "xyz", path: path, thumbnail_path: nil, type: .file))

        return x
    }
    override func update(_ event: AppEvent) async {}

    override func update_view(_ model: AppViewModel) {}
}

