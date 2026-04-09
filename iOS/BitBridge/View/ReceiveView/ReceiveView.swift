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

    @State var receiveSession: ReceiveSessionViewModel?
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
                            .padding(.top, safeAreaInsets.top)
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

                    if let session = self.receiveSession {
                        VStack(spacing: SpaceTheme.item.value) {
                            ReceiveSessionHeaderView(session: session, selectedItem: $selectedItem)
                            ReceiveSessionBodyView(session: session)
                        }
                        .padding(.horizontal, SpaceTheme.screen.value)
                        .padding(.top, SpaceTheme.item.value)
                    }

                    Spacer().frame(height: 160)
                }
            }
            .mask(MaskTheme.Bottom)
            .padding(.bottom, SpaceTheme.screen.value)

        }
        .ignoresSafeArea()
        .onReceive(self.core.transfer, perform: { value in
            self.receiveSession = value?.received_session
        })
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)

}
