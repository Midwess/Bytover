//
//  ListItem.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 23/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct SelectedResourceItem: View {
    @State var resource: LocalResource
    
    func getThumbnail() -> some View {
        switch resource.type {
        case .file: return Group {
            Image(systemName: "doc.fill")
        } as Group
        case .folder: return Group {
            Image(systemName: "folder.fill")
        } as Group
        case .image: return Group {
            Image(systemName: "photo.fill")
        } as Group
        case .video: return Group {
            Image(systemName: "video.fill")
        } as Group
        case .other: return Group {
            Image(systemName: "questionmark")
        } as Group
        }
    }
    
    var body: some View {
        HStack(alignment: .center, spacing: 5) {
            getThumbnail()
                .font(.title3)
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
            }
            Button(action: {}) {
                Image(systemName: "trash")
                    .font(.callout)
                    .opacity(0.7)
                    .fontWeight(.bold)
                    .foregroundColor(Theme.LightViolet.color.opacity(0.8))
                    .padding(.leading, 12)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 12)
        .background(Theme.LightViolet.color.opacity(0.00))
        .clipShape(Capsule())
        .overlay(Capsule().stroke(Theme.LightViolet.color.opacity(0.0), lineWidth: 1))
    }
}

#Preview {
    ZStack {
        StunningBackgroundGradientAnimation()
        SelectedResourceItem(resource: CoreMock.withSelectedFileTransfers().transfer!.selected_resources[0])
            .padding()
    }
}
