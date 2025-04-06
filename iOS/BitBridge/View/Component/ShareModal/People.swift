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
    var peer: PeerViewModel
    
    var body: some View {
        HStack(spacing: 12) {
            Avartar(avatar: peer.avatar)
                .frame(width: 42, height: 42)
            VStack(alignment: .leading, spacing: 4) {
                Text(peer.display_name)
                    .foregroundColor(Theme.PrimaryText.color)
                    .modifier(Label1())
                
                Text("Nearby")
                    .modifier(Label2())
                    .padding(.trailing, 8)
                    .foregroundColor(Theme.PrimaryText.color.opacity(0.7))
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity)
        .clipShape(Capsule())
        .background(Capsule().fill(Theme.PrimaryText.color.opacity(0.1)))
        .overlay(Capsule().stroke(Theme.PrimaryText.color.opacity(0.1)))
    }
}

struct PeopleShareView: View {
    @EnvironmentObject private var core: Core
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
            
            if core.transfer?.nearby_peers.isEmpty ?? true {
                Text("No nearby friends found")
                    .modifier(Label1())
                    .foregroundStyle(Theme.PrimaryText.color)
                    .padding()
            }
            else {
                ScrollView(.horizontal) {
                    LazyHStack(alignment: .top) {
                        ForEach(core.transfer?.nearby_peers ?? [], id: \.self) { peer in
                            PeopleShareItem(peer: peer)
                        }
                    }
                }
                .frame(minWidth: 100, minHeight: 70)
                .scrollIndicators(.hidden)
            }
        }
    }
}

#Preview {
    PeopleShareView()
        .environmentObject(CoreMock.empty() as Core)
}
