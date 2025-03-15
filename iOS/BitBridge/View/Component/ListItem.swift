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
    @EnvironmentObject private var core: Core
    @State var isVisible = false
    
    func getDefaultThumbnail() -> some View {
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
        case .image:
            return AnyView(
                ImageAsset.FileImage.image
                    .resizable()
                    .frame(width: 32, height: 32)
            )
        case .video:
            return AnyView(
                ImageAsset.CameraVideo.image
                    .resizable()
                    .frame(width: 32, height: 32)
            )
        case .other:
            return AnyView(
                ImageAsset.File.image
                    .resizable()
                    .frame(width: 32, height: 32)
            )
        }
    }
    
    func getThumbnail() -> some View {
        if isVisible {
            if let thumbnail_path = resource.thumbnail_path {
                if let thumbnail_image = Image.fromRelativePath(thumbnail_path) {
                    return AnyView(thumbnail_image.resizable()
                        .scaledToFill()
                        .frame(width: 48, height: 48).cornerRadius(14))
                }
            }
        }
        
        return AnyView(ZStack {
            Rectangle()
                .frame(width: 48, height: 48)
                .cornerRadius(14)
                .foregroundStyle(getColor())
            getDefaultThumbnail()
                .frame(width: 32, height: 32)
        })
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
            getThumbnail()
        }
        .onAppear {
            isVisible = true
        }
        .onDisappear {
            isVisible = false
        }
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
                
                if resource.size_gb <= 0 {
                    Text("\(String(resource.size_mb)) MB")
                        .modifier(Label1())
                }
                else {
                   Text("\(String(resource.size_mb)) MB")
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
                                Task {
                                    await core.update(.transfer(.removeResource(resource.order_id)))
                                }
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
        .onAppear {
            isVisible = true
        }
        .onDisappear {
            isVisible = false
        }
    }
}

#Preview {
    ZStack {
        StunningBackgroundGradientAnimation()
        SelectedResourceItem(resource: CoreMock.withSelectedFileTransfers().transfer!.selected_resources[0])
            .padding()
    }
}
