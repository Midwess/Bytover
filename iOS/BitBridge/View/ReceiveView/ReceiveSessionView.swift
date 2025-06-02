//
//  ReceiveSession.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 10/5/25.
//

import Foundation
import SwiftUI
import SharedTypes

struct ImageReceiveResourceView: View {
    var session_id: UInt64
    @State var localResource: ImageReceiveResourceViewModel
    @State var isShowingMoreDialog: Bool = false
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core
    
    var body: some View {
        GeometryReader { geometry in
            HStack {
                ResourceImage(resource: localResource.model, width: geometry.size.width, height: geometry.size.height, radius: 20)
            }
            .onReceive(core.transfer, perform: { value in
                guard let session = value?.received_sessions.first(where: { item in item.id == session_id}) else {
                    return
                }
                
                guard let resource = session.image_resources.first(where: { resource in resource.model.order_id == localResource.model.order_id}) else {
                    return;
                }
                
                let thumbnailChanges = resource.model.thumbnail_path != self.localResource.model.thumbnail_path
                
                if thumbnailChanges {
                    self.localResource = resource
                }
            })
            .onTapGesture {
                Task {
                    await core.update(.transfer(.openSessionResource(session_id: session_id, resource_id: localResource.model.order_id)))
                }
            }
            .onAppearAndReceive(core.transfer, perform: { value in
                guard let itemValue = value!.received_sessions.first(where: { item in item.id == session_id})?.image_resources.first(where: { resource in resource.model.order_id == localResource.model.order_id}) else {
                    return;
                }
                
                if itemValue != self.localResource {
                    self.localResource = itemValue
                }
            })
        }
    }
}

struct FileReceiveResourceView: View {
    var sessionId: UInt64
    @State var localResource: FileReceiveResourceViewModel
    @State var isShowingMoreDialog: Bool = false
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core

    var body: some View {
        HStack {
            ResourceImage(resource: localResource.model, backgroundColor: false)
                .background(
                    RoundedRectangle(cornerRadius: 15)
                        .fill(Theme.PrimaryText.color.opacity(0.05))
                )
            VStack(alignment: .leading, spacing: 5) {
                Text(localResource.model.name)
                    .modifier(Label1())
                
                Text(localResource.model.display_path)
                    .modifier(Label3())
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .opacity(0.7)
            }
            .padding(.trailing, SpaceTheme.item.value)
            
            Spacer()
            if localResource.model.size_mb > 0 {
                Text("\(String(localResource.model.size_mb)) MB")
                    .modifier(Label1())
            }
            else if localResource.model.size_gb > 0 {
                Text("\(String(localResource.model.size_gb)) GB")
                    .modifier(Label1())
            }
        }
        .frame(idealWidth: screenSize.width - 80, maxWidth: 320, idealHeight: 45)
        .padding(.horizontal, 10)
        .padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 20)
                .fill(Theme.PrimaryText.color.opacity(0.1))
        )
        .onTapGesture {
            Task {
                await core.update(.transfer(.openSessionResource(session_id: sessionId, resource_id: localResource.model.order_id)))
            }
        }
        .onAppearAndReceive(core.transfer, perform: { value in
            guard let itemValue = value!.received_sessions.first(where: { item in item.id == sessionId})?.file_resources.first(where: { resource in resource.model.order_id == localResource.model.order_id}) else {
                return;
            }
            
            if itemValue != self.localResource {
                self.localResource = itemValue
            }
        })
    }
}

struct ReceiveSessionBodyView: View {
    @EnvironmentObject var core: Core
    @State var session: ReceiveSessionViewModel
    @Environment(\.screenSize) private var screenSize

    private let flexibleColumn = [
        GridItem(.flexible(minimum: 70)),
    ]
    
    
    private let flexibleColumn2 = [
        GridItem(.flexible(minimum: 70)),
        GridItem(.flexible(minimum: 70)),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            if self.session.image_resources.count > 0 {
                VStack(alignment: .leading, spacing: SpaceTheme.cohesive.value) {
                    Text("\(self.session.image_resources.count) Image\(self.session.image_resources.count > 1 ? "s" : "")")
                        .modifier(Caption())
                        .foregroundColor(Theme.PrimaryText.color)
                    ScrollView(.horizontal) {
                        let width = ((screenSize.width - 80) / CGFloat(min(self.session.image_resources.count, 3))).rounded();
                        let height = min(width * 1.3, 140);
                        LazyHGrid(rows: [GridItem(.flexible(minimum: 140))], spacing: 10) {
                            ForEach(self.session.image_resources, id: \.model.order_id) { item in
                                ImageReceiveResourceView(session_id: self.session.id, localResource: item)
                                    .frame(width: width, height: height)
                            }
                        }
                    }
                }
            }
            
            if self.session.file_resources.count > 0 {
                VStack(alignment: .leading, spacing: SpaceTheme.cohesive.value) {
                    Text("\(self.session.file_resources.count) File\(self.session.file_resources.count > 1 ? "s" : "")")
                        .modifier(Caption())
                        .foregroundColor(Theme.PrimaryText.color)
                    ScrollView(.horizontal) {
                        LazyHGrid(rows: self.session.file_resources.count > 3 ? flexibleColumn2 : flexibleColumn, spacing: 10) {
                            ForEach(self.session.file_resources, id: \.model.order_id) { item in
                                FileReceiveResourceView(sessionId: session.id, localResource: item)
                            }
                        }
                    }
                }
            }
            
            Divider()
        }
        .onReceive(self.core.transfer, perform: { value in
            guard let receivedSession = value!.received_sessions.first(
                where: {session in session.id == self.session.id}) else {
                return
            }
            
            if receivedSession.file_resources.count != self.session.file_resources.count || receivedSession.image_resources.count != self.session.image_resources.count || receivedSession.video_resources.count != self.session.video_resources.count {
                self.session = receivedSession
            }
        })
    }
}

struct ReceiveSessionHeaderView: View {
    @EnvironmentObject var core: Core
    @State var session: ReceiveSessionViewModel
    @Binding var isShowMoreOption: Bool
    @Binding var selectedItem: ReceiveSessionViewModel?
    
    var body: some View {
        HStack(alignment: .center, spacing: SpaceTheme.cohesive.value) {
            Avartar(avatar: session.peer_avatar)
                .frame(width: 42, height: 42)
                .padding(.trailing, SpaceTheme.cohesive.value)
            VStack(alignment: .leading, spacing: SpaceTheme.cohesive.value - 3) {
                Text(session.peer_name)
                    .foregroundColor(Theme.PrimaryText.color)
                    .modifier(Label1())
                Text(session.display_datetime)
                    .modifier(Label2())
            }
            Spacer()
            if session.is_in_progress {
                ZStack(alignment: .trailing) {
                    Text("0000000.0 MB/s")
                        .modifier(Label2())
                        .opacity(0)
                    HStack(spacing: SpaceTheme.cohesive.value) {
                        Text(session.display_download_speed)
                            .modifier(Label2())
                            .foregroundColor(Theme.PrimaryText.color)
                        ImageAsset.Download.image
                            .offset(x: -2)
                            .scaleEffect(1.2)
                            .foregroundColor(Theme.BluePrimary.color)
                        CircularProgressView(progress: session.progress)
                            .frame(width: 30, height: 30)
                            .onTapGesture { action in
                                Task {
                                    await core.update(.transfer(.cancelTransfer(session_id: session.id)))
                                }
                            }
                    }
                }
            }
            else {
                MoreOptionButton<ReceiveSessionViewModel>(state: $isShowMoreOption, item: session, selectedItem: $selectedItem)
            }
        }
        .onReceive(self.core.transfer, perform: { value in
            guard let receivedSession = value!.received_sessions.first(
                where: {session in session.id == self.session.id}) else {
                return
            }
            
            if receivedSession.progress != self.session.progress {
                self.session = receivedSession
            }
            
            if receivedSession.is_in_progress != self.session.is_in_progress && receivedSession.is_completed != self.session.is_completed {
                self.session = receivedSession
            }
            
            if (receivedSession.display_download_speed != self.session.display_download_speed) {
                self.session = receivedSession
            }
        })
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)
}
