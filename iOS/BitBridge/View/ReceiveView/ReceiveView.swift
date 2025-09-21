//
//  ReceiveView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct ReceiveView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core

    @State var receiveSessions: [ReceiveSessionViewModel] = []
    @State var selectedItem: ReceiveSessionViewModel?
    @State var isShowItemOption = false

    var body: some View {
        ZStack {
            StunningBackgroundGradient()
            ScrollView {
                LazyVStack(spacing: SpaceTheme.screen.value) {
                    VStack(spacing: SpaceTheme.item.value) {
                        LogoScene(gltfFileName: "Rocket", logoScale: 1.6)
                            .frame(width: screenSize.width, height: 100)
                            .overlay(Theme.gradientHeading
                                .opacity(0.5)
                                .blur(radius: 15)
                                .frame(width: screenSize.width, height: screenSize.width / 2)
                            )

                        Text("Your Inbox")
                            .padding(.horizontal, 20)
                            .multilineTextAlignment(.center)
                            .modifier(Heading1())

                        UpgradePremiumButton()
                    }

                    VStack(spacing: SpaceTheme.item.value) {
                        ForEach(self.receiveSessions, id: \.self.id) { item in
                            ReceiveSessionHeaderView(session: item, isShowMoreOption: $isShowItemOption, selectedItem: $selectedItem)
                            ReceiveSessionBodyView(session: item)
                        }
                        .padding(.horizontal, SpaceTheme.screen.value)
                        .padding(.top, SpaceTheme.item.value)
                    }

                    Spacer().frame(height: 160)
                }
            }
            .mask(LinearGradient(gradient: Gradient(colors: [.black, .black, .black, .black, .clear]), startPoint: .top, endPoint: .bottom).opacity(0.9))
            .padding(.bottom, SpaceTheme.screen.value)

        }
        .confirmationDialog(selectedItem?.peer_name ?? "Session", isPresented: $isShowItemOption) {
            Button("Open") {
                Task {
                    await core.update(.transfer(.openSession(session_id: selectedItem?.id ?? 0)))
                }
            }

            Button("Delete", role: .destructive) {
                Task {
                    await core.update(.transfer(.deleteSession(session_id: selectedItem?.id ?? 0)))
                }
            }

        }
        .onReceive(self.core.transfer, perform: { value in
            let receivedSessions = value?.received_sessions ?? []

            if receivedSessions.count != self.receiveSessions.count {
                self.receiveSessions = receivedSessions
            }
        })
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)

}
