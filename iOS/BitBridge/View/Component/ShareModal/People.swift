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
        HStack(spacing: 0) {
            Avartar(avatar: peer.avatar)
                .frame(width: 42, height: 42)
                .padding(.trailing, 12)
            VStack(alignment: .leading, spacing: 4) {
                Text(peer.display_name)
                    .foregroundColor(Theme.PrimaryText.color)
                    .modifier(Label1())
                HStack(spacing: 2) {
                    Text("Nearby")
                        .modifier(Label2())
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.7))
                    
                    if let uploadSpeed = peer.display_upload_speed {
                        ZStack(alignment: .trailing) {
                            Text("00.00 MB/s")
                                .font(.caption)
                                .opacity(0)
                            Text(uploadSpeed)
                                .font(.caption)
                                .foregroundColor(Theme.PrimaryText.color)
                        }
                        ImageAsset.Upload.image
                            .padding(.leading, 1)
                            .scaleEffect(1)
                            .foregroundColor(Theme.BluePrimary.color)
                        }
                    
                    if let downloadSpeed = peer.display_download_speed {
                        ZStack(alignment: .trailing) {
                            Text("00.00 MB/s")
                                .font(.caption)
                                .opacity(0)
                            Text(downloadSpeed)
                                .font(.caption)
                                .foregroundColor(Theme.PrimaryText.color)
                        }
                        ImageAsset.Download.image
                            .padding(.leading, 1)
                            .scaleEffect(1)
                            .foregroundColor(Theme.BluePrimary.color)
                    }
                    
                    Spacer()
                }
            }
            
            if peer.transfer_progress > 0 {
                SharingProgress(progress: peer.transfer_progress)
                    .frame(width: 38, height: 38)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 10)
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
        VStack(spacing: 16) {
            Button(action: {}) {
                HStack {
                    Text("Find or add new people here")
                        .modifier(Label1())
                        .foregroundStyle(Theme.PrimaryText.color)
                    Spacer()
                }
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity)
            .background(Theme.PrimaryText.color.opacity(0.15))
            .clipShape(Capsule())
            
            if nearbyPeers.isEmpty {
                Text("No nearby friends found")
                    .modifier(Label1())
                    .foregroundStyle(Theme.PrimaryText.color)
                    .padding()
            }
            else {
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
                .frame(minWidth: 100, minHeight: 70)
                .scrollIndicators(.hidden)
            }
        }
        .onReceive(self.core.transfer, perform: { value in
            if value?.nearby_peers.count ?? 0 != nearbyPeers.count {
                nearbyPeers = value?.nearby_peers ?? []
            }
        })
    }
}

#Preview {
    PeopleShareView()
        .environmentObject(CoreMock.empty() as Core)
}
