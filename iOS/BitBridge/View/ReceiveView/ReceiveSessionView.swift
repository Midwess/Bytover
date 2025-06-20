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
    var width: CGFloat
    var height: CGFloat
    @State var localResource: ImageReceiveResourceViewModel
    @State var isShowingMoreDialog: Bool = false
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core
    
    var body: some View {
        Button(action: {
            Task {
                await core.update(.transfer(.openSessionResource(session_id: session_id, resource_id: localResource.model.order_id)))
            }
        }) {
            ZStack {
                ResourceImage(resource: localResource.model, width: width, height: height, radius: 20)
                
                
                if !localResource.is_completed {
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle(tint: Theme.PrimaryText.color.opacity(0.9)))
                        .scaleEffect(1.5)
                        .background(Theme.BlackBase.color.blur(radius: 20))
                }
            }
        }
        .onReceive(core.transfer, perform: { value in
            guard let itemValue = value!.received_sessions.first(where: { item in item.id == session_id})?.image_resources.first(where: { resource in resource.model.order_id == localResource.model.order_id}) else {
                return;
            }
            
            if itemValue != self.localResource {
                self.localResource = itemValue
            }
        })
        .frame(width: width, height: height)
    }
}

struct VideoReceiveResourceView: View {
    var session_id: UInt64
    var width: CGFloat
    var height: CGFloat
    @State var localResource: VideoReceiveResourceViewModel
    @State var isShowingMoreDialog: Bool = false
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core
    
    var body: some View {
        Button(action: {
            Task {
                await core.update(.transfer(.openSessionResource(session_id: session_id, resource_id: localResource.model.order_id)))
            }
        }) {
            ZStack {
                ResourceImage(resource: localResource.model, width: width, height: height, radius: 20)
                
                if !localResource.is_completed {
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle(tint: Theme.PrimaryText.color.opacity(0.9)))
                        .scaleEffect(1.5)
                        .background(Theme.BlackBase.color.blur(radius: 20))
                }
            }
        }
        .frame(width: width, height: height)
        .onAppearAndReceive(core.transfer, perform: { value in
            guard let itemValue = value!.received_sessions.first(where: { item in item.id == session_id})?.video_resources.first(where: { resource in resource.model.order_id == localResource.model.order_id}) else {
                return;
            }
            
            if itemValue != self.localResource {
                self.localResource = itemValue
            }
        })
    }
}

struct FileReceiveResourceView: View {
    var sessionId: UInt64
    @State var localResource: FileReceiveResourceViewModel
    @State var isShowingMoreDialog: Bool = false
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core

    var body: some View {
        Button(action: {
            Task {
                await core.update(.transfer(.openSessionResource(session_id: sessionId, resource_id: localResource.model.order_id)))
            }
        }) {
            ZStack {
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
                
                if !localResource.is_completed {
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle(tint: Theme.PrimaryText.color.opacity(0.9)))
                        .scaleEffect(1.5)
                        .background(Theme.BlackBase.color.blur(radius: 20))
                }
            }
            .frame(idealWidth: screenSize.width - 80, maxWidth: 320, idealHeight: 45)
            .padding(.horizontal, 10)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 20)
                    .fill(Theme.PrimaryText.color.opacity(0.1))
            )
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
            let mediaCount = self.session.image_resources.count + self.session.video_resources.count;
            if mediaCount > 0 {
                VStack(alignment: .leading, spacing: SpaceTheme.cohesive.value) {
                    HStack(spacing: 8) {
                        if self.session.video_resources.count > 0 {
                            Text("\(self.session.video_resources.count) Video\(self.session.video_resources.count > 1 ? "s" : "")")
                                .modifier(Caption())
                                .foregroundColor(Theme.PrimaryText.color)
                        }
                        if self.session.image_resources.count > 0 {
                            Text("\(self.session.image_resources.count) Image\(self.session.image_resources.count > 1 ? "s" : "")")
                                .modifier(Caption())
                                .foregroundColor(Theme.PrimaryText.color)
                        }
                    }
                    ScrollView(.horizontal) {
                        let width = ((screenSize.width - SpaceTheme.screen.value * 2) / CGFloat(min(mediaCount, 3)) - 10).rounded();
                        let height = min(width * 1.3, 140);
                        LazyHGrid(rows: [GridItem(.flexible(minimum: 140))], spacing: 10) {
                            ForEach(self.session.video_resources, id: \.model.order_id) { item in
                                VideoReceiveResourceView(session_id: self.session.id, width: width, height: height, localResource: item)
                                    .frame(width: width, height: height)
                            }
                            ForEach(self.session.image_resources, id: \.model.order_id) { item in
                                ImageReceiveResourceView(session_id: self.session.id, width: width, height: height, localResource: item)
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
