//
//  ListItem.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 23/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct ResourceImage: View {
    var resource: LocalResource
    @State private var thumbnail: UIImage?
    @State private var isLoading: Bool = false
    @EnvironmentObject private var core: Core
    
    func getThumbnail() -> some View {
        switch resource.type {
        case .file:
            return AnyView(
                ImageAsset.File.image
                    .resizable()
                    .frame(width: 32, height: 32)
            )
        case .folder:
            return AnyView(
                ImageAsset.Folder.image
                    .resizable()
                    .frame(width: 32, height: 32)
            )
        case .image, .video:
            if let thumbnail = thumbnail {
                return AnyView(
                    Image(uiImage: thumbnail)
                        .resizable()
                        .scaledToFill()
                        .frame(width: 48, height: 48)
                        .clipShape(RoundedRectangle(cornerRadius: 14))
                )
            }
           else {
                let icon = resource.type == .image ?
                    ImageAsset.FileImage.image : ImageAsset.CameraVideo.image
                return AnyView(
                    icon
                        .resizable()
                        .frame(width: 32, height: 32)
                )
            }
        case .other:
            return AnyView(
                ImageAsset.File.image
                    .resizable()
                    .frame(width: 32, height: 32)
            )
        }
    }
    
    func getColor() -> Color {
        switch resource.type {
        case .file: return Theme.FileColor.color
        case .folder: return Theme.FolderColor.color
        case .image: return Theme.DocumentColor.color
        case .video: return Theme.DocumentColor.color
        case .other: return Theme.DocumentColor.color
        }
    }
    
    var body: some View {
        ZStack {
            if thumbnail == nil {
                Rectangle()
                    .frame(width: 48, height: 48)
                    .cornerRadius(14)
                    .foregroundStyle(getColor())
            }
            
            getThumbnail()
        }
        .onAppear {
            loadThumbnail()
        }
    }
    
    private func loadThumbnail() {
        if (resource.type == .image || resource.type == .video) &&
            !isLoading &&
            thumbnail == nil {
            
            let identifier = core.displayResourcePath(path: resource.path)
            isLoading = true
            
            core.getMediaThumbnail(for: identifier, size: CGSize(width: 96, height: 96)) { image in
                self.thumbnail = image
                self.isLoading = false
            }
        }
    }
}

struct SelectedResourceItem: View {
    @State var resource: LocalResource
    @State var isShowingMoreDialog: Bool = false
    @EnvironmentObject var core: Core
    
    var body: some View {
        HStack(alignment: .center, spacing: 7) {
            ResourceImage(resource: resource)
                .foregroundColor(.black.opacity(0.5))
            VStack(alignment: .leading, spacing: 5) {
                Text(resource.name)
                    .modifier(Label1())
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text(core.displayResourcePath(path: resource.path))
                    .modifier(Label3())
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .opacity(0.7)
            }
            .padding(.leading, 10)
            Spacer()
            VStack(alignment: .trailing, spacing: 7) {
                if core.bytesToGB(bytesLength: Float(resource.size)) > 0 {
                    Text(String(core.bytesToGB(bytesLength: Float(resource.size))) + " GB")
                        .modifier(Label1())
                }
                
                if core.bytesToGB(bytesLength: Float(resource.size)) <= 0 {
                    Text(String(core.bytesToMB(bytesLength: Float(resource.size))) + " MB")
                        .modifier(Label1())
                }
                else {
                   Text(String(core.bytesToMB(bytesLength: Float(resource.size))) + " MB")
                        .modifier(Label2())
                        .opacity(0.7)
                }
            }
            Button(action: {isShowingMoreDialog = true}) {
                ImageAsset.More.image
                    .scaleEffect(1.6)
                    .confirmationDialog(
                        "\(resource.name)",
                        isPresented: self.$isShowingMoreDialog) {
                            Button("Remove") {
                            }
                            Button("Open") {
                            }
                        }
            }
            .frame(minWidth: 35, alignment: .trailing)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 10)
        .background(.gray.opacity(0.0))
        .clipShape(RoundedRectangle(cornerRadius: 15))
        .overlay(RoundedRectangle(cornerRadius: 15).stroke(.white.opacity(0.0), lineWidth: 1))
    }
}

#Preview {
    ZStack {
        StunningBackgroundGradientAnimation()
        SelectedResourceItem(resource: CoreMock.withSelectedFileTransfers().transfer!.selected_resources[0])
            .padding()
    }
}
