//
//  FilePickerView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 22/2/25.
//

import Foundation
import SwiftUI
import PhotosUI

struct MediaPickerView: UIViewControllerRepresentable {
    @Environment(\.presentationMode) var presentationMode
    @State private var selectedItems: [MediaItem] = []
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    func makeUIViewController(context: UIViewControllerRepresentableContext<MediaPickerView>) -> PHPickerViewController {
        var config = PHPickerConfiguration()
        config.selectionLimit = 0 // 0 means no limit
        config.filter = .any(of: [.images, .videos])
        
        let picker = PHPickerViewController(configuration: config)
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: PHPickerViewController, context: UIViewControllerRepresentableContext<MediaPickerView>) {
    }
    
    class Coordinator: NSObject, PHPickerViewControllerDelegate {
        let parent: MediaPickerView

        init(_ parent: MediaPickerView) {
            self.parent = parent
        }
        
        func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            parent.presentationMode.wrappedValue.dismiss()
            
            for result in results {
                let itemProvider = result.itemProvider
                
                if itemProvider.canLoadObject(ofClass: UIImage.self) {
                    // Handle image
                    itemProvider.loadObject(ofClass: UIImage.self) { image, error in
                        if let image = image as? UIImage {
                            DispatchQueue.main.async {
                                self.parent.selectedItems.append(MediaItem(type: .image, image: image))
                            }
                        }
                    }
                } else if itemProvider.hasItemConformingToTypeIdentifier(UTType.movie.identifier) {
                    // Handle video
                    itemProvider.loadFileRepresentation(forTypeIdentifier: UTType.movie.identifier) { url, error in
                        if let url = url {
                            // Create a copy of the video in app's document directory
                            let fileName = url.lastPathComponent
                            let documentsURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                            let destinationURL = documentsURL.appendingPathComponent(fileName)
                            
                            try? FileManager.default.copyItem(at: url, to: destinationURL)
                            
                            DispatchQueue.main.async {
                                self.parent.selectedItems.append(MediaItem(type: .video, videoURL: destinationURL))
                            }
                        }
                    }
                }
            }
        }
    }
}

// Media type enum
enum MediaType {
    case image
    case video
}

// Media item struct
struct MediaItem: Identifiable {
    let id = UUID()
    let type: MediaType
    let image: UIImage?
    let videoURL: URL?
    
    init(type: MediaType, image: UIImage? = nil, videoURL: URL? = nil) {
        self.type = type
        self.image = image
        self.videoURL = videoURL
    }
}

#Preview {
    MediaPickerView()
}
