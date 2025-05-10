//
//  ReceiveSession.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 10/5/25.
//

import Foundation
import SwiftUI
import SharedTypes

struct ReceiveSessionView: View {
    @EnvironmentObject var core: Core
    @State var session: ReceiveSessionViewModel
    
    var body: some View {
        ZStack(alignment: .leading) {
            VStack(alignment: .leading) {
                HStack(alignment: .center) {
                    Avartar(avatar: session.peer_avatar)
                        .frame(width: 42, height: 42)
                        .padding(.trailing, 12)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(session.peer_name)
                            .foregroundColor(Theme.PrimaryText.color)
                            .modifier(Label1())
                        Text(session.peer_description)
                            .modifier(Label2())
                    }
                    Spacer()
                    if session.is_in_progress {
                        ZStack(alignment: .trailing) {
                            Text("00000.0 MB/s")
                                .modifier(Label2())
                                .opacity(0)
                            HStack {
                                Text(session.display_download_speed)
                                    .modifier(Label1())
                                    .foregroundColor(Theme.PrimaryText.color)
                                ImageAsset.Download.image
                                    .offset(x: -1)
                                    .scaleEffect(1.7)
                                    .foregroundColor(Theme.BluePrimary.color)
                                CircularProgressView(progress: session.progress)
                                    .frame(width: 32, height: 32)
                                    .padding(.leading, 4)
                            }
                        }
                    }
               }
            }
        }
        .padding()
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)
}
