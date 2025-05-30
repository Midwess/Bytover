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
    var resource: SelectedResourceViewModel
    var width: CGFloat = 48
    var height: CGFloat = 48
    var radius: CGFloat? = nil
    @EnvironmentObject private var core: Core
    @State private var thumbnailImage: Image?
    
    static func getDefaultThumbnail(_ type: ResourceType) -> some View {
        switch type {
        case .file:
            return AnyView(
                ImageAsset.File.image
                    .resizable()
                    .scaledToFit()
            )
        case .image:
            return AnyView(
                ImageAsset.FileImage.image
                    .resizable()
                    .scaledToFit()
            )
        case .video:
            return AnyView(
                ImageAsset.CameraVideo.image
                    .resizable()
                    .scaledToFit()
            )
        case .folder:
            return AnyView(
                ImageAsset.Folder.image
                    .resizable()
                    .scaledToFit()
            )
        }
    }
    
    func getThumbnail() -> some View {
        if let thumbnail_path = resource.thumbnail_path {
            if let image = thumbnailImage {
                return AnyView(image.resizable()
                    .frame(width: width, height: height)
                    .cornerRadius(radius ?? ((width + height) / 2) * 0.3))
            } else {
                // Load the image asynchronously
                Task {
                    thumbnailImage = await Image.fromPath(thumbnail_path, core: core)
                }
            }
        }
        
        return AnyView(ZStack {
            Rectangle()
                .frame(width: width, height: height)
                .cornerRadius(radius ?? ((width + height) / 2) * 0.3)
                .foregroundStyle(getColor())
            ResourceImage.getDefaultThumbnail(resource.type)
                .padding(((width + height) / 2) * 0.1)
                .frame(width: width, height: height)
        })
    }
    
    func getColor() -> Color {
        switch resource.type {
        case .file: return Theme.FileColor.color
        case .image: return Theme.DocumentColor.color
        case .video: return Theme.DocumentColor.color
        case .folder: return Theme.Navy.color
        }
    }
    
    var body: some View {
        getThumbnail()
    }
}

struct SelectedResourceItem: View {
    @State var resource: SelectedResourceViewModel
    @State var isShowingMoreDialog: Bool = false
    @State private var isVisible: Bool = false
    @EnvironmentObject var core: Core
    
    var body: some View {
        HStack(alignment: .center, spacing: 7) {
            ResourceImage(resource: resource)
                .foregroundColor(.black.opacity(0.5))
            
            VStack(alignment: .leading, spacing: 5) {
                Text(resource.name)
                    .strikethrough(!resource.is_valid)
                    .modifier(Label1())
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text(resource.display_path)
                    .modifier(Label3())
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .opacity(0.7)
            }
            .padding(.leading, 10)
            Spacer()
            VStack(alignment: .trailing, spacing: 7) {
                if resource.size_gb > 0 {
                    Text("\(String(resource.size_gb)) GB")
                        .modifier(Label1())
                }
                
                if resource.size_mb >= 0 {
                    Text("\(String(resource.size_mb)) MB")
                        .modifier(Label1())
                }
            }
            Button(action: {isShowingMoreDialog = true}) {
                ImageAsset.More.image
                    .scaleEffect(1.6)
                    .confirmationDialog(
                        "\(resource.name)",
                        isPresented: self.$isShowingMoreDialog) {
                            Button("Remove") {
                                Task {
                                    await core.update(.transfer(.removeResource(resource.order_id)))
                                }
                            }
                        }
            }
            .frame(minWidth: 35, alignment: .trailing)
        }
        .onTapGesture {
            print("Open")
            Task {
                await core.update(.transfer(.openSelectedResource(resource_id: resource.order_id)))
            }
        }
        .opacity(resource.is_valid ? 1 : 0.5)
        .background(.gray.opacity(0.0))
        .clipShape(RoundedRectangle(cornerRadius: 15))
        .overlay(RoundedRectangle(cornerRadius: 15).stroke(.white.opacity(0.0), lineWidth: 1))
        .onAppear {
            isVisible = true
        }
        .onDisappear {
            isVisible = false
        }
        .onReceive(self.core.transfer, perform: { value in
            let newResource = value?.selected_resources.first(where: { resource in resource.order_id == self.resource.order_id }) ?? self.resource
            if newResource != self.resource {
                self.resource = newResource
            }
        })
    }
}

#Preview {
    ZStack {
        SelectedResourceItem(resource: CoreMock.withSelectedFileTransfers().transfer.value!.selected_resources[0])
            .padding()
    }
}
