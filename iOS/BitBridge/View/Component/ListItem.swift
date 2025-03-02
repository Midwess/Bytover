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
    
    func getThumbnail() -> some View {
        switch resource.type {
        case .file: return
            ImageAsset.File.image
            .resizable()
                .frame(width: 32, height: 32)
        case .folder: return
            ImageAsset.Folder.image
                .resizable()
                .frame(width: 32, height: 32)
        case .image: return
            ImageAsset.FileImage.image
                .resizable()
                .frame(width: 32, height: 32)
        case .video: return
            ImageAsset.CameraVideo.image
                .resizable()
                .frame(width: 32, height: 32)
        case .other: return
            ImageAsset.File.image
                .resizable()
                .frame(width: 32, height: 32)
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
            Rectangle()
                .frame(width: 50, height: 50)
                .cornerRadius(15)
                .foregroundStyle(getColor())
            getThumbnail()
                .frame(width: 50, height: 50)
        }
    }
}

struct SelectedResourceItem: View {
    @State var resource: LocalResource
    
    var body: some View {
        HStack(alignment: .center, spacing: 5) {
            ResourceImage(resource: resource)
                .foregroundColor(.black.opacity(0.5))
            VStack(alignment: .leading, spacing: 5) {
                Text(resource.name)
                    .modifier(Label1())
                Text("/Users/tiendang")
                    .modifier(Label3())
                    .opacity(0.7)
            }
            .padding(.leading, 10)
            Spacer()
            VStack(spacing: 5) {
                Text("0.1 GB")
                    .modifier(Label1())
                Text("100 MB")
                    .modifier(Label3())
                    .opacity(0.7)
            }
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
