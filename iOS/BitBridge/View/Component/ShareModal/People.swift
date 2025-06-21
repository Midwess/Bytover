//
//  People.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 6/3/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct PeopleShareItem: View {
    @EnvironmentObject private var core: Core
    @State var peer: PeerViewModel

    var body: some View {
        HStack(spacing: SpaceTheme.cohesive.value) {
            Avartar(avatar: peer.avatar)
                .frame(width: 42, height: 42)
                .padding(.trailing, SpaceTheme.cohesive.value)
            VStack(alignment: .leading, spacing: SpaceTheme.cohesive.halfValue) {
                Text(peer.display_name)
                    .foregroundColor(Theme.PrimaryText.color)
                    .modifier(Label1())
                HStack(spacing: SpaceTheme.cohesive.halfValue) {
                    Text("Nearby")
                        .modifier(Label2())
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.8))

                    if let uploadSpeed = peer.display_upload_speed {
                        ZStack(alignment: .leading) {
                            Text("000000.0 MB/s")
                                .modifier(Label2())
                                .opacity(0)
                            HStack {
                                Text(uploadSpeed)
                                    .modifier(Label2())
                                    .foregroundColor(Theme.PrimaryText.color)
                                ImageAsset.Upload.image
                                    .offset(x: -1)
                                    .foregroundColor(Theme.BluePrimary.color)
                            }
                        }
                    }
                }
            }

            if peer.display_upload_speed != nil {
                SharingProgress(progress: peer.transfer_progress)
                    .frame(width: 40, height: 40)
            }
        }
        .padding(.horizontal, SpaceTheme.item.value)
        .padding(.vertical, SpaceTheme.cohesive.value)
        .frame(maxWidth: .infinity)
        .clipShape(Capsule())
        .background(Capsule().fill(Theme.PrimaryText.color.opacity(0.1)))
        .overlay(Capsule().stroke(Theme.PrimaryText.color.opacity(0.1)))
        .onReceive(self.core.transfer, perform: { value in
            let newPeer = value?.nearby_peers.first(where: {peer in peer.id == self.peer.id}) ?? self.peer
            if self.peer != newPeer {
                self.peer = newPeer
            }
        })
    }
}

struct PeopleShareView: View {
    @EnvironmentObject private var core: Core
    @State private var nearbyPeers: [PeerViewModel] = []

    var body: some View {
        VStack(spacing: SpaceTheme.item.value) {
            Button(action: {}) {
                HStack {
                    Text("Find or add new people here")
                        .modifier(Label1())
                        .foregroundStyle(Theme.PrimaryText.color.opacity(0.8))
                    Spacer()
                }
            }
            .padding(.vertical, SpaceTheme.item.value)
            .padding(.horizontal, SpaceTheme.screen.value)
            .frame(maxWidth: .infinity)
            .background(Theme.PrimaryText.color.opacity(0.15))
            .clipShape(Capsule())

            if nearbyPeers.isEmpty {
                Text("No nearby friends found")
                    .modifier(Label1())
                    .foregroundStyle(Theme.PrimaryText.color)
                    .padding()
            } else {
                ScrollView(.horizontal) {
                    LazyHStack(alignment: .top) {
                        ForEach(nearbyPeers, id: \.self) { peer in
                            Button(action: {
                                Task {
                                    await core.update(.transfer(.startTransfer(target_id: peer.id)))
                                }
                            }) {
                                PeopleShareItem(peer: peer)
                            }
                        }
                    }
                }
                .frame(minWidth: 100, minHeight: 80)
                .scrollIndicators(.hidden)
            }
        }
        .onAppearAndReceive(self.core.nearby, perform: { value in
            if value?.peers.count ?? 0 != nearbyPeers.count {
                nearbyPeers = value?.peers ?? []
            }
        })
    }
}

#Preview {
    PeopleShareView()
        .environmentObject(CoreMock.empty() as Core)
}
