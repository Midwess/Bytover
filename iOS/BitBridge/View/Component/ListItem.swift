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
    @State var resource: ResourceSelection
    
    func getThumbnail() -> some View {
        switch resource.type {
        case .file: return Group {
            Image(systemName: "doc")
                .font(.title2)
        } as Group
        case .folder: return Group {
            Image(systemName: "folder")
                .font(.title2)
        } as Group
        case .image: return Group {
            Image(systemName: "photo")
                .font(.title2)
        } as Group
        case .video: return Group {
            Image(systemName: "video")
                .font(.title2)
        } as Group
        case .other: return Group {
            Image(systemName: "questionmark")
                .font(.title2)
        } as Group
        }
    }
    
    var body: some View {
        HStack(alignment: .center) {
            getThumbnail()
                .frame(width: 50, height: 50)
                .foregroundColor(Theme.LightViolet.color.opacity(0.8))
                .background(
                    Rectangle()
                        .foregroundStyle(Theme.gradientHeading2)
                        .cornerRadius(15)
                )
            VStack(alignment: .leading) {
                Text(resource.name)
                    .modifier(Label1())
                Text("10 MB")
                    .modifier(Label3())
                    .opacity(0.7)
            }
            .padding(.leading, 10)
            Spacer()
            Button(action: {}) {
                Image(systemName: "trash")
                    .font(.callout)
                    .fontWeight(.regular)
                    .foregroundColor(Theme.LightViolet.color.opacity(0.6))
            }
        }
    }
}

#Preview {
    SelectedResourceItem(resource: ResourceSelection(data: .localPath("path"), type: .image, name: "Name"))
}
