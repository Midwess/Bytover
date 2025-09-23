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
    var radius: CGFloat?
    var backgroundColor: Bool = true

    @EnvironmentObject private var core: Core
    @State private var thumbnailImage: Image?

    var body: some View {
        ZStack {
            if let thumbnailImage = thumbnailImage {
                thumbnailImage
                    .resizable()
                    .scaledToFill()
                    .frame(width: width, height: height)
                    .cornerRadius(radius ?? ((width + height) / 2) * 0.3)
            } else {
                Rectangle()
                    .frame(width: width, height: height)
                    .cornerRadius(radius ?? ((width + height) / 2) * 0.3)
                    .foregroundStyle(getColor())
                ResourceImage.getDefaultThumbnail(resource.type)
                    .padding(width != .infinity && height != .infinity ? ((width + height) / 2) * 0.1 : 0)
                    .frame(width: width, height: height)
            }

            // Video overlay icon
            if resource.type == .video {
                HStack {
                    Spacer()
                    VStack {
                        Spacer()
                        ZStack {
                            Circle()
                                .fill(Theme.BlackBase.color)
                                .blur(radius: min(width, height) * 0.16)
                                .frame(width: min(width, height) * 0.2, height: min(width, height) * 0.2)
                            ImageAsset.CameraVideoSolid.image
                                .resizable()
                                .scaledToFit()
                                .foregroundColor(Theme.PrimaryText.color.opacity(0.82))
                                .padding(min(width, height) * 0.1)
                        }
                        .frame(width: min(width, height) * 0.40, height: min(width, height) * 0.40)
                    }
                }
                .frame(width: width, height: height)
                .padding(4)
            }
        }
        .onAppearOrChange(of: resource.thumbnail_path) { oldValue, newValue in
            Task {
                if oldValue != newValue {
                    await loadThumbnail()
                }
            }
        }
    }

    func getColor() -> Color {
        if !backgroundColor {
            return .clear
        }
        switch resource.type {
        case .file: return Theme.FileColor.color
        case .image: return Theme.DocumentColor.color
        case .video: return Theme.DocumentColor.color
        case .folder: return Theme.Navy.color
        }
    }

    static func getDefaultThumbnail(_ type: ResourceType) -> some View {
        switch type {
        case .file:
            return AnyView(ImageAsset.File.image.resizable().scaledToFit())
        case .image:
            return AnyView(ImageAsset.FileImage.image.resizable().scaledToFit())
        case .video:
            return AnyView(ImageAsset.CameraVideo.image.resizable().scaledToFit())
        case .folder:
            return AnyView(ImageAsset.Folder.image.resizable().scaledToFit())
        }
    }

    private func loadThumbnail() async {
        guard thumbnailImage == nil, let thumbnail_path = resource.thumbnail_path else { return }
        thumbnailImage = await Image.fromPath(thumbnail_path, core: core)
    }
}

struct SelectedResourceItem: View {
    @State var resource: SelectedResourceViewModel
    @State var isShowMoreOption: Bool = false
    @Binding var selectedItem: SelectedResourceViewModel?
    @State private var isVisible: Bool = false
    @EnvironmentObject var core: Core

    var body: some View {
        HStack(alignment: .center, spacing: SpaceTheme.cohesive.value) {
            ResourceImage(resource: resource)
                .foregroundColor(.black.opacity(0.6))

            VStack(alignment: .leading, spacing: SpaceTheme.cohesive.value) {
                Text(resource.name)
                    .modifier(Label1())
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text(resource.display_path)
                    .modifier(Label2())
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .opacity(0.8)
            }
            .padding(.leading, SpaceTheme.item.value)
            Spacer()
            VStack(alignment: .trailing, spacing: SpaceTheme.cohesive.value) {
                if resource.size_gb > 0 {
                    Text("\(String(resource.size_gb)) GB")
                        .modifier(Label1())
                }

                if resource.size_mb >= 0 {
                    Text("\(String(resource.size_mb)) MB")
                        .modifier(Label1())
                }
            }
            MoreOptionButton<SelectedResourceViewModel>(state: $isShowMoreOption, item: resource, selectedItem: $selectedItem)
        }
        .onTapGesture {
            Task {
                await core.update(.transfer(.openSelectedResource(resource_id: resource.order_id)))
            }
        }
        .opacity(1)
        .background(.gray.opacity(0.0))
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).stroke(.white.opacity(0.0), lineWidth: 1))
        .onAppear {
            isVisible = true
        }
        .onDisappear {
            isVisible = false
        }
        .confirmationDialog(
            resource.name,
            isPresented: $isShowMoreOption) {
                Button("Remove", role: .destructive) {
                    Task {
                        await core.update(.transfer(.removeResource(resource.order_id)))
                    }
                }
            }
        .onReceive(self.core.transfer, perform: { value in
            let newResource = value?.selected_resources.first(where: { resource in resource.order_id == self.resource.order_id }) ?? self.resource
            if newResource != self.resource {
                self.resource = newResource
            }
        })
    }
}
